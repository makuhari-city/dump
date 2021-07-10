use actix_web::client::Client;
use serde_json::Value;

const IPFS_LOG_BASE_URL: &str = "https://vote.metacity.jp/ipfs/log";

pub async fn post_ipfs(data: &Value) -> Option<String> {
    let endpoint = format!("{}/", IPFS_LOG_BASE_URL);

    let client = Client::new();
    let response = client
        .post(&endpoint)
        .header("Content-Type", "application.json")
        .send_json(data)
        .await;

    if response.is_err() {
        log::error!("could not post to ipfs endpoint");
        return None;
    }

    let data = response.unwrap().body().await;

    match data {
        Ok(d) => return Some(String::from_utf8(d.to_vec()).unwrap()),
        _ => return None,
    }
}
