use crate::pinboard;

pub async fn post_to_tg_channel(pin: pinboard::Pin) -> Result<(), Box<dyn std::error::Error>> {
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
            Some(n) if !n.is_empty() => format!("\n\n{}", n),
            _ => "".to_string(),
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
