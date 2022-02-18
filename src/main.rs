use serde::Deserialize;

const PINBOARD_POPULAR_ENDPOINT: &str = "http://feeds.pinboard.in/json/popular/";

#[derive(Deserialize, Debug)]
struct Pin {
    u: String,
    d: String,
    t: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pins = reqwest::get(PINBOARD_POPULAR_ENDPOINT)
        .await?
        .json::<Vec<Pin>>()
        .await?;
    Ok(())
}
