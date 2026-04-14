use reqwest::Client;
use serde_json::json;
use anyhow::Result;

pub async fn send_verification_code(chat_id: i64, code: &str, bot_token: &str) -> Result<()> {
    let client = Client::new();
    let url = format!("https://api.telegram.org/bot{}/sendMessage", bot_token);
    let payload = json!({
        "chat_id": chat_id,
        "text": format!("Ваш код подтверждения: <b>{}</b>\n\nНе передавайте его никому!", code),
        "parse_mode": "HTML"
    });

    let res = client.post(&url).json(&payload).send().await?;
    if !res.status().is_success() {
        let err = res.text().await?;
        anyhow::bail!("Telegram API error: {}", err);
    }
    Ok(())
}
