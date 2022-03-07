use serde::Deserialize;

const PINBOARD_POPULAR_ENDPOINT: &str = "http://feeds.pinboard.in/json/popular/";

#[derive(Deserialize, Debug)]
pub struct Pin {
    pub u: String,
    pub d: String,
    pub n: Option<String>,
    pub a: String,
    pub t: Vec<String>,
}

pub async fn fetch_pins() -> Result<Vec<Pin>, Box<dyn std::error::Error>> {
    let mut pins = reqwest::get(PINBOARD_POPULAR_ENDPOINT)
        .await?
        .json::<Vec<Pin>>()
        .await?;
    // put the latest at the tail
    pins.reverse();
    Ok(pins)
}
