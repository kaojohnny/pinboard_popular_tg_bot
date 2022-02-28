use rusqlite::{params, Connection, Result};
use serde::Deserialize;
use tokio_cron_scheduler::{Job, JobScheduler};

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
    let text = format!(
        "{}\n\n{}{}{}",
        pin.d,
        pin.u,
        match pin.n {
            Some(n) => format!("\n\n{}", n),
            None => "".to_string(),
        },
        if pin.t.len() > 0 {
            format!("\n\n{}", pin.t.join(", "))
        } else {
            "".to_string()
        }
    );
    client
        .get(path)
        .query(&[("chat_id", dotenv!("TG_CHAT_ID")), ("text", &text.trim())])
        .send()
        .await?;
    Ok(())
}

fn from_storage() -> Result<(u32, Pin), Box<dyn std::error::Error>> {
    let conn = Connection::open(dotenv!("DB_FILE"))?;

    let mut stmt = conn
        .prepare("SELECT id, title, author, link, description, tags  FROM 'pins' WHERE sent IS NULL ORDER BY id LIMIT 1")?;
    let mut pins = stmt.query_map([], |row| {
        let tags_string: String = row.get(5).unwrap();
        let id: u32 = row.get(0)?;
        Ok((
            id,
            Pin {
                d: row.get(1)?,
                a: row.get(2)?,
                u: row.get(3)?,
                n: row.get(4)?,
                t: tags_string.split(',').map(|s| s.to_string()).collect(),
            },
        ))
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

fn set_pin_sent_to_storage(id: u32) -> Result<(), Box<dyn std::error::Error>> {
    let conn = Connection::open(dotenv!("DB_FILE"))?;
    conn.execute("UPDATE pins SET sent = 1 WHERE id = ?", [id])?;

    Ok(())
}

async fn handle_pull() -> Result<(), Box<dyn std::error::Error>> {
    let pins = fetch_pins().await?;
    to_storage(&pins)?;

    Ok(())
}

async fn handle_push() -> Result<(), Box<dyn std::error::Error>> {
    let (id, pin) = from_storage()?;
    post_to_tg_channel(pin).await?;
    set_pin_sent_to_storage(id)?;

    Ok(())
}

#[tokio::main]
async fn main() {
    let mut sched = JobScheduler::new();

    // every 15 mins
    sched
        .add(
            Job::new_async("0 0/15 * * * ? *", |_uuid, _l| {
                Box::pin(async {
                    handle_push().await.expect("handle_push failed");
                })
            })
            .unwrap(),
        )
        .expect("Add to JobScheduler failed");

    // every 6 hours and 1 min
    sched
        .add(
            Job::new_async("0 27 0 * * ? *", |_uuid, _l| {
                Box::pin(async {
                    handle_pull().await.expect("handle_pull failed");
                })
            })
            .unwrap(),
        )
        .expect("Add to JobScheduler failed");

    #[cfg(feature = "signal")]
    sched.shutdown_on_ctrl_c();

    sched
        .set_shutdown_handler(Box::new(|| {
            Box::pin(async move {
                println!("Shut down done");
            })
        }))
        .expect("Set shutdown handler failed");

    sched.start().await.expect("JobScheduler starts failed");
}
