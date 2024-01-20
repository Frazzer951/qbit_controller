use qbit_rs::model::Credential;
use qbit_rs::Qbit;

#[tokio::main]
async fn main() {
    let credential = Credential::new("admin", "adminadmin");
    let api = Qbit::new("http://10.10.20.20:8282/", credential);
    let torrents = api.get_version().await;

    match torrents {
        Ok(torrents) => {
            println!("{torrents}");
        }
        Err(e) => println!("Error: {}", e),
    }
}
