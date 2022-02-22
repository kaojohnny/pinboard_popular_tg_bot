use rusqlite::{params, Connection, Result};
use serde::Deserialize;

#[macro_use]
extern crate dotenv_codegen;

const PINBOARD_POPULAR_ENDPOINT: &str = "http://feeds.pinboard.in/json/popular/";

#[derive(Deserialize, Debug)]
struct Pin {
    u: String,
    d: String,
    n: Option<String>,
    a: String,
    t: Vec<String>,
}

async fn fetch_pins() -> Result<Vec<Pin>, Box<dyn std::error::Error>> {
    let mut pins = reqwest::get(PINBOARD_POPULAR_ENDPOINT)
        .await?
        .json::<Vec<Pin>>()
        .await?;
    // put the latest at the tail
    pins.reverse();
    Ok(pins)
}

fn to_storage(pins: &Vec<Pin>) -> Result<(), Box<dyn std::error::Error>> {
    let mut conn = Connection::open(dotenv!("DB_FILE"))?;

    conn.execute(
        "create table if not exists pins (
             id integer primary key,
             title text not null,
             author text not null,
             link text not null,
             description text,
             tags text,
             sent integer,
             UNIQUE(author, link)
         )",
        [],
    )?;

    let tx = conn.transaction()?;
    for pin in pins.iter() {
        // pinboard returns something like "t":[""]
        let tags: Vec<String> = pin
            .t
            .iter()
            .filter(|t| !t.is_empty())
            .map(|t| ["#", t].join(""))
            .collect();
        tx.execute(
            "INSERT OR IGNORE INTO pins (title, author, link, description, tags) values (?1, ?2, ?3, ?4, ?5)",
            params![&pin.d, &pin.a, &pin.u, &pin.n, &tags.join(", ")],
        )?;
    }
    tx.commit()?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pins = fetch_pins().await?;

    to_storage(&pins)?;

    Ok(())
}
