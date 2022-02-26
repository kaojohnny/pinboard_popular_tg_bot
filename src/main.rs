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

async fn post_to_tg_channel(pin: Pin) -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let path = format!(
        "https://api.telegram.org/bot{}/sendMessage",
        dotenv!("TG_BOT_TOKEN")
    );
    let text = format!("{}\n{}\n{}", pin.d, pin.u, pin.t.join(", "));
    client
        .get(path)
        .query(&[("chat_id", dotenv!("TG_CHAT_ID")), ("text", &text)])
        .send()
        .await?;
    Ok(())
}

fn from_storage() -> Result<Pin, Box<dyn std::error::Error>> {
    let conn = Connection::open(dotenv!("DB_FILE"))?;

    let mut stmt = conn
        .prepare("SELECT title, author, link, description, tags  FROM 'pins' WHERE sent IS NULL ORDER BY id LIMIT 1")?;
    let mut pins = stmt.query_map([], |row| {
        let tags_string: String = row.get(4).unwrap();
        Ok(Pin {
            d: row.get(0)?,
            a: row.get(1)?,
            u: row.get(2)?,
            n: row.get(3)?,
            t: tags_string.split(',').map(|s| s.to_string()).collect(),
        })
    })?;

    Ok(pins.nth(0).unwrap()?)
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
    let op = std::env::args().nth(1).expect("no operation given");

    if op == "pull" {
        let pins = fetch_pins().await?;
        to_storage(&pins)?;
    } else if op == "push" {
        let p = from_storage()?;
        post_to_tg_channel(p).await?;
    } else {
        panic!("only support pull for now");
    }

    Ok(())
}
