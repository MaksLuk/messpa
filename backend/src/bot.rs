use teloxide::{prelude::*, types::ParseMode};
use redis::AsyncCommands;
use serde_json::json;
use std::sync::Arc;
use crate::state::AppState;
use crate::utils::token::generate_code;

pub async fn run_telegram_bot(state: Arc<AppState>) {
    let bot = Bot::new(state.config.telegram_bot_token.clone());

    teloxide::repl(bot, move |bot: Bot, msg: Message| {
        let state = state.clone();
        async move {
            if let Some(text) = msg.text() {
                if let Some(magic_token) = text.strip_prefix("/start auth_") {
                    handle_auth_start(bot.clone(), msg.clone(), magic_token, state.clone()).await;
                }
                if let Some(magic_token) = text.strip_prefix("verify_") {
                    handle_verify(bot.clone(), msg.clone(), magic_token, state).await;
                }
            }
            Ok(())
        }
    })
    .await;
}

async fn handle_auth_start(bot: Bot, msg: Message, magic_token: &str, state: Arc<AppState>) {
    let mut redis = state.redis_conn.clone();
    let redis_key = format!("pending_auth:{}", magic_token);

    let data_str: Option<String> = redis.get(&redis_key).await.ok();
    let Some(data_str) = data_str else {
        bot.send_message(msg.chat.id, "Ссылка устарела. Запросите новую.").await.ok();
        return;
    };

    let mut data: serde_json::Value = serde_json::from_str(&data_str).unwrap();

    let code = generate_code();
    data["code"] = json!(code);
    data["telegram_chat_id"] = json!(msg.chat.id.0);

    redis.set_ex::<&str, String, ()>(&redis_key, data.to_string(), 900).await.ok();

    let text = format!(
        "🔐 Ваш код подтверждения:\n\n<b>{}</b>\n\nВставьте его на сайте.",
        code
    );

    bot.send_message(msg.chat.id, text)
        .parse_mode(ParseMode::Html)
        .await
        .ok();

    bot.send_message(msg.chat.id, "После ввода кода на сайте вы будете автоматически авторизованы.").await.ok();
}

async fn handle_verify(bot: Bot, msg: Message, magic_token: &str, state: Arc<AppState>) {
    let mut redis = state.redis_conn.clone();
    let redis_key = format!("pending_telegram:{}", magic_token);

    let data_str: Option<String> = redis.get(&redis_key).await.ok();
    let Some(data_str) = data_str else {
        bot.send_message(msg.chat.id, "Ссылка устарела. Запросите новую.").await.ok();
        return;
    };

    let mut data: serde_json::Value = serde_json::from_str(&data_str).unwrap();

    let code = generate_code();
    data["code"] = json!(code);
    data["telegram_chat_id"] = json!(msg.chat.id.0);

    redis.set_ex::<&str, String, ()>(&redis_key, data.to_string(), 900).await.ok();

    let text = format!(
        "🔐 Ваш код подтверждения:\n\n<b>{}</b>\n\nВставьте его на сайте.",
        code
    );

    bot.send_message(msg.chat.id, text)
        .parse_mode(ParseMode::Html)
        .await
        .ok();

    bot.send_message(msg.chat.id, "После ввода кода на сайте вы будете автоматически авторизованы.").await.ok();
}
