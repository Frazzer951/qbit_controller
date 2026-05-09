use std::collections::HashSet;

use anyhow::{Result, anyhow};

use crate::{
    config::{ControllerConfig, DurationValue},
    qbit_api::Torrent,
};

pub fn dry_run_prefix(config: &ControllerConfig) -> &'static str {
    if config.settings.dry_run {
        "[DRY-RUN] "
    } else {
        ""
    }
}

pub fn torrent_hash(torrent: &Torrent) -> Result<String> {
    torrent
        .hash
        .clone()
        .ok_or_else(|| anyhow!("Torrent is missing a hash"))
}

pub fn parse_tags(tags: &Option<String>) -> HashSet<String> {
    tags.as_deref()
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

pub fn is_complete(torrent: &Torrent) -> bool {
    torrent.progress.unwrap_or_default() >= 1.0
        || matches!(
            torrent.state.as_deref(),
            Some("uploading")
                | Some("pausedUP")
                | Some("stoppedUP")
                | Some("queuedUP")
                | Some("stalledUP")
                | Some("checkingUP")
                | Some("forcedUP")
        )
}

pub fn parse_duration_minutes(value: &DurationValue) -> Result<i64> {
    match value {
        DurationValue::Minutes(minutes) => Ok(*minutes),
        DurationValue::Text(text) => parse_duration_text(text),
    }
}

fn parse_duration_text(text: &str) -> Result<i64> {
    let text = text.trim();
    if text.is_empty() {
        return Err(anyhow!("Duration cannot be empty"));
    }

    if let Ok(minutes) = text.parse::<i64>() {
        return Ok(minutes);
    }

    let mut number = String::new();
    let mut total = 0_i64;

    for ch in text.chars() {
        if ch.is_ascii_digit() {
            number.push(ch);
            continue;
        }

        if number.is_empty() {
            return Err(anyhow!("Invalid duration '{text}'"));
        }

        let amount: i64 = number.parse()?;
        number.clear();

        total += match ch {
            'm' => amount,
            'h' => amount * 60,
            'd' => amount * 24 * 60,
            'w' => amount * 7 * 24 * 60,
            _ => return Err(anyhow!("Invalid duration unit '{ch}' in '{text}'")),
        };
    }

    if !number.is_empty() {
        return Err(anyhow!("Duration '{text}' is missing a unit"));
    }

    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_comma_separated_tags() {
        let tags = parse_tags(&Some("one, two,,three ".to_owned()));
        assert_eq!(
            tags,
            HashSet::from_iter(["one".into(), "two".into(), "three".into()])
        );
    }

    #[test]
    fn parses_duration_units() {
        assert_eq!(parse_duration_text("30d").unwrap(), 43_200);
        assert_eq!(parse_duration_text("1w3d2h32m").unwrap(), 14_552);
        assert_eq!(parse_duration_text("-1").unwrap(), -1);
    }

    #[test]
    fn rejects_missing_duration_unit() {
        assert!(parse_duration_text("10h5").is_err());
    }
}
