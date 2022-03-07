use crate::pinboard;
use rusqlite::{params, Connection, Result};

pub fn from_storage() -> Result<(u32, pinboard::Pin), Box<dyn std::error::Error>> {
    let conn = Connection::open(dotenv!("DB_FILE"))?;

    let mut stmt = conn
        .prepare("SELECT id, title, author, link, description, tags  FROM 'pins' WHERE sent IS NULL ORDER BY id LIMIT 1")?;
    let mut pins = stmt.query_map([], |row| {
        let tags_string: String = row.get(5).unwrap();
        let id: u32 = row.get(0)?;
        Ok((
            id,
            pinboard::Pin {
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

pub fn to_storage(pins: &Vec<pinboard::Pin>) -> Result<(), Box<dyn std::error::Error>> {
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

pub fn set_pin_sent_to_storage(id: u32) -> Result<(), Box<dyn std::error::Error>> {
    let conn = Connection::open(dotenv!("DB_FILE"))?;
    conn.execute("UPDATE pins SET sent = 1 WHERE id = ?", [id])?;

    Ok(())
}
