use anyhow::{Context, Result, anyhow};
use reqwest::{Client, StatusCode, Url};
use serde::{Deserialize, Serialize};

use crate::config;

#[derive(Clone)]
pub struct QbitClient {
    client: Client,
    base_url: Url,
    username: String,
    password: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Torrent {
    pub added_on: Option<i64>,
    pub category: Option<String>,
    pub hash: Option<String>,
    pub name: Option<String>,
    pub num_complete: Option<i64>,
    pub progress: Option<f64>,
    pub ratio: Option<f64>,
    pub ratio_limit: Option<f64>,
    pub seeding_time: Option<i64>,
    pub seeding_time_limit: Option<i64>,
    pub state: Option<String>,
    pub tags: Option<String>,
    pub tracker: Option<String>,
    pub up_limit: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Tracker {
    pub url: String,
    pub status: i8,
    #[serde(default)]
    pub msg: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RatioLimit {
    Global,
    NoLimit,
    Limited(f64),
}

impl Serialize for RatioLimit {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Global => serializer.serialize_i64(-2),
            Self::NoLimit => serializer.serialize_i64(-1),
            Self::Limited(limit) => serializer.serialize_f64(*limit),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MinuteLimit {
    Global,
    NoLimit,
    Limited(u64),
}

impl Serialize for MinuteLimit {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Global => serializer.serialize_i64(-2),
            Self::NoLimit => serializer.serialize_i64(-1),
            Self::Limited(limit) => serializer.serialize_u64(*limit),
        }
    }
}

impl QbitClient {
    pub fn new(qbit: &config::Qbit) -> Result<Self> {
        let mut url = qbit.url.clone();
        if !url.starts_with("http://") && !url.starts_with("https://") {
            url = format!("http://{url}");
        }
        if !url.ends_with('/') {
            url.push('/');
        }

        Ok(Self {
            client: Client::builder().cookie_store(true).build()?,
            base_url: Url::parse(&url).context("Invalid qBittorrent URL")?,
            username: qbit.username.clone(),
            password: qbit.password.clone(),
        })
    }

    pub async fn login(&self) -> Result<()> {
        #[derive(Serialize)]
        struct Login<'a> {
            username: &'a str,
            password: &'a str,
        }

        let response = self
            .client
            .post(self.url("auth/login")?)
            .form(&Login {
                username: &self.username,
                password: &self.password,
            })
            .send()
            .await?;

        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        if status == StatusCode::OK && body.trim() == "Ok." {
            return Ok(());
        }

        Err(anyhow!(
            "qBittorrent login failed with status {status}: {body}"
        ))
    }

    pub async fn get_version(&self) -> Result<String> {
        self.get_text("app/version").await
    }

    pub async fn get_webapi_version(&self) -> Result<String> {
        self.get_text("app/webapiVersion").await
    }

    pub async fn get_torrents(&self) -> Result<Vec<Torrent>> {
        self.get_json("torrents/info").await
    }

    pub async fn get_trackers(&self, hash: &str) -> Result<Vec<Tracker>> {
        #[derive(Serialize)]
        struct Arg<'a> {
            hash: &'a str,
        }

        self.get_json_with("torrents/trackers", &Arg { hash }).await
    }

    pub async fn add_tags(&self, hashes: &[String], tags: &[String]) -> Result<()> {
        if hashes.is_empty() || tags.is_empty() {
            return Ok(());
        }

        #[derive(Serialize)]
        struct Arg {
            hashes: String,
            tags: String,
        }

        self.post_form(
            "torrents/addTags",
            &Arg {
                hashes: hashes.join("|"),
                tags: tags.join(","),
            },
        )
        .await
    }

    pub async fn remove_tags(&self, hashes: &[String], tags: &[String]) -> Result<()> {
        if hashes.is_empty() || tags.is_empty() {
            return Ok(());
        }

        #[derive(Serialize)]
        struct Arg {
            hashes: String,
            tags: String,
        }

        self.post_form(
            "torrents/removeTags",
            &Arg {
                hashes: hashes.join("|"),
                tags: tags.join(","),
            },
        )
        .await
    }

    pub async fn set_share_limits(
        &self,
        hashes: &[String],
        ratio_limit: RatioLimit,
        seeding_time_limit: MinuteLimit,
        inactive_seeding_time_limit: MinuteLimit,
    ) -> Result<()> {
        if hashes.is_empty() {
            return Ok(());
        }

        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Arg {
            hashes: String,
            ratio_limit: RatioLimit,
            seeding_time_limit: MinuteLimit,
            inactive_seeding_time_limit: MinuteLimit,
        }

        self.post_form(
            "torrents/setShareLimits",
            &Arg {
                hashes: hashes.join("|"),
                ratio_limit,
                seeding_time_limit,
                inactive_seeding_time_limit,
            },
        )
        .await
    }

    pub async fn set_upload_limit(
        &self,
        hashes: &[String],
        limit_bytes_per_second: u64,
    ) -> Result<()> {
        if hashes.is_empty() {
            return Ok(());
        }

        #[derive(Serialize)]
        struct Arg {
            hashes: String,
            limit: u64,
        }

        self.post_form(
            "torrents/uploadLimit",
            &Arg {
                hashes: hashes.join("|"),
                limit: limit_bytes_per_second,
            },
        )
        .await
    }

    pub async fn start_torrents(&self, hashes: &[String]) -> Result<()> {
        if hashes.is_empty() {
            return Ok(());
        }

        #[derive(Serialize)]
        struct Arg {
            hashes: String,
        }

        self.post_form(
            "torrents/start",
            &Arg {
                hashes: hashes.join("|"),
            },
        )
        .await
    }

    pub async fn delete_torrents(&self, hashes: &[String], delete_files: bool) -> Result<()> {
        if hashes.is_empty() {
            return Ok(());
        }

        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Arg {
            hashes: String,
            delete_files: bool,
        }

        self.post_form(
            "torrents/delete",
            &Arg {
                hashes: hashes.join("|"),
                delete_files,
            },
        )
        .await
    }

    pub async fn set_auto_management(&self, hashes: &[String], enable: bool) -> Result<()> {
        if hashes.is_empty() {
            return Ok(());
        }

        #[derive(Serialize)]
        struct Arg {
            hashes: String,
            enable: bool,
        }

        self.post_form(
            "torrents/setAutoManagement",
            &Arg {
                hashes: hashes.join("|"),
                enable,
            },
        )
        .await
    }

    pub async fn set_category(&self, hashes: &[String], category: &str) -> Result<()> {
        if hashes.is_empty() {
            return Ok(());
        }

        #[derive(Serialize)]
        struct Arg<'a> {
            hashes: String,
            category: &'a str,
        }

        self.post_form(
            "torrents/setCategory",
            &Arg {
                hashes: hashes.join("|"),
                category,
            },
        )
        .await
    }

    fn url(&self, path: &str) -> Result<Url> {
        self.base_url
            .join(&format!("api/v2/{path}"))
            .with_context(|| format!("Invalid qBittorrent API path: {path}"))
    }

    async fn get_text(&self, path: &str) -> Result<String> {
        let response = self.client.get(self.url(path)?).send().await?;
        ensure_success(response)
            .await?
            .text()
            .await
            .map_err(Into::into)
    }

    async fn get_json<T>(&self, path: &str) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let response = self.client.get(self.url(path)?).send().await?;
        ensure_success(response)
            .await?
            .json()
            .await
            .map_err(Into::into)
    }

    async fn get_json_with<T, A>(&self, path: &str, arg: &A) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
        A: Serialize + ?Sized,
    {
        let response = self.client.get(self.url(path)?).query(arg).send().await?;
        ensure_success(response)
            .await?
            .json()
            .await
            .map_err(Into::into)
    }

    async fn post_form<A>(&self, path: &str, arg: &A) -> Result<()>
    where
        A: Serialize + ?Sized,
    {
        let response = self.client.post(self.url(path)?).form(arg).send().await?;
        ensure_success(response).await?;
        Ok(())
    }
}

async fn ensure_success(response: reqwest::Response) -> Result<reqwest::Response> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }

    let body = response.text().await.unwrap_or_default();
    Err(anyhow!(
        "qBittorrent API request failed with status {status}: {body}"
    ))
}
