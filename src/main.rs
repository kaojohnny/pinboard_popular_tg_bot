mod pinboard;
mod storage;
mod tg;

#[macro_use]
extern crate dotenv_codegen;

async fn handle_pull() -> Result<(), Box<dyn std::error::Error>> {
    let pins = pinboard::fetch_pins().await?;
    storage::to_storage(&pins)?;

    Ok(())
}

async fn handle_push() -> Result<(), Box<dyn std::error::Error>> {
    let (id, pin) = storage::from_storage()?;
    tg::post_to_tg_channel(pin).await?;
    storage::set_pin_sent_to_storage(id)?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    handle_pull().await?;
    handle_push().await?;

    Ok(())
}
