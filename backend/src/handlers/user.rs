use axum::{
    extract::{State, Extension},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use chrono::Utc;
use diesel::prelude::*;
use redis::AsyncCommands;

use std::sync::Arc;

use crate::{
    state::AppState,
    models::user::{User, Language, Currency},
    schema::users,
    error::AppError,
    utils::{mail, token},
};

#[derive(Deserialize)]
pub struct UpdateDisplayNamePayload {
    pub display_name: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateLanguagePayload {
    pub language: Language,
}

#[derive(Deserialize)]
pub struct UpdateCurrencyPayload {
    pub currency: Currency,
}

#[derive(Deserialize)]
pub struct InitiateEmailPayload {
    pub email: String,
}

#[derive(Deserialize)]
pub struct VerifyPayload {
    pub magic_token: String,
    pub code: String,
}

#[derive(Serialize)]
pub struct EmailResponse {
    pub magic_token: String,
    pub message: String,
}

#[derive(Serialize)]
pub struct TelegramResponse {
    pub deep_link: String,
    pub magic_token: String,
    pub message: String,
}

// GET /api/user/me
pub async fn get_current_user(
    Extension(user): Extension<User>,
) -> impl IntoResponse {
    Json(serde_json::json!({
        "ok": true,
        "data": user
    }))
}

// PATCH /api/user/display-name
pub async fn update_display_name(
    State(state): State<Arc<AppState>>,
    Extension(mut user): Extension<User>,
    Json(payload): Json<UpdateDisplayNamePayload>,
) -> Result<impl IntoResponse, AppError> {
    let mut conn = state.db_pool.get().map_err(|e| AppError::Pool(e.into()))?;

    diesel::update(users::table.find(user.id))
        .set(users::display_name.eq(&payload.display_name))
        .execute(&mut conn)?;

    user.display_name = payload.display_name;

    Ok(Json(serde_json::json!({"ok": true, "data": user})))
}

// PATCH /api/user/language
pub async fn update_language(
    State(state): State<Arc<AppState>>,
    Extension(mut user): Extension<User>,
    Json(payload): Json<UpdateLanguagePayload>,
) -> Result<impl IntoResponse, AppError> {
    let mut conn = state.db_pool.get().map_err(|e| AppError::Pool(e.into()))?;

    diesel::update(users::table.find(user.id))
        .set(users::language.eq(&payload.language))
        .execute(&mut conn)?;

    user.language = payload.language;

    Ok(Json(serde_json::json!({"ok": true, "data": user})))
}

// PATCH /api/user/currency
pub async fn update_currency(
    State(state): State<Arc<AppState>>,
    Extension(mut user): Extension<User>,
    Json(payload): Json<UpdateCurrencyPayload>,
) -> Result<impl IntoResponse, AppError> {
    let mut conn = state.db_pool.get().map_err(|e| AppError::Pool(e.into()))?;

    diesel::update(users::table.find(user.id))
        .set(users::currency.eq(&payload.currency))
        .execute(&mut conn)?;

    user.currency = payload.currency;

    Ok(Json(serde_json::json!({"ok": true, "data": user})))
}

// POST /api/user/email
pub async fn initiate_set_email(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    Json(payload): Json<InitiateEmailPayload>,
) -> Result<impl IntoResponse, AppError> {
    if user.email.is_some() {
        return Err(AppError::Validation("Email уже установлен".into()));
    }

    let magic_token = token::generate_magic_token();
    let code = token::generate_code();

    let redis_key = format!("pending_email:{}", magic_token);

    let data = serde_json::json!({
        "user_id": user.id.to_string(),
        "email": payload.email,
        "code": code,
        "created_at": Utc::now().timestamp()
    });

    let mut redis = state.redis_conn.clone();
    redis.set_ex(&redis_key, data.to_string(), 900)
        .await
        .map_err(|e| AppError::Redis(e))?;

    mail::send_mail(
        &state.config.smtp_host,
        state.config.smtp_port,
        &state.config.email_addr,
        &payload.email,
        "Подтверждение email",
        format!("Код подтверждения: {}", code),
    ).await
    .map_err(|e| AppError::Validation(format!("Не удалось отправить письмо: {}", e)))?;

    Ok(Json(serde_json::json!({
        "ok": true,
        "data": EmailResponse {
            magic_token,
            message: "Код отправлен на ваш email".to_string()
        }
    })))
}

// POST /api/user/email/verify
pub async fn verify_set_email(
    State(state): State<Arc<AppState>>,
    Extension(mut user): Extension<User>,
    Json(payload): Json<VerifyPayload>,
) -> Result<impl IntoResponse, AppError> {
    let redis_key = format!("pending_email:{}", payload.magic_token);
    let mut redis = state.redis_conn.clone();

    let data_str: Option<String> = redis.get(&redis_key).await.ok();
    let data_str = data_str.ok_or_else(|| AppError::Validation("Код истёк или недействителен".into()))?;

    let data: serde_json::Value = serde_json::from_str(&data_str)
        .map_err(|_| AppError::Validation("Ошибка данных".into()))?;

    if data["code"].as_str() != Some(&payload.code) {
        return Err(AppError::Validation("Неверный код".into()));
    }

    let email = data["email"].as_str()
        .ok_or(AppError::Validation("Ошибка данных".into()))?
        .to_string();

    let mut conn = state.db_pool.get().map_err(|e| AppError::Pool(e.into()))?;

    diesel::update(users::table.find(user.id))
        .set(users::email.eq(Some(email.clone())))
        .execute(&mut conn)?;

    user.email = Some(email);

    redis.del::<&str, ()>(&redis_key).await.ok();

    Ok(Json(serde_json::json!({
        "ok": true,
        "data": user
    })))
}

// POST /api/user/telegram
pub async fn initiate_set_telegram(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
) -> Result<impl IntoResponse, AppError> {
    if user.telegram_id.is_some() {
        return Err(AppError::Validation("Telegram уже установлен".into()));
    }

    let magic_token = token::generate_magic_token();
    let redis_key = format!("pending_telegram:{}", magic_token);
    let data = serde_json::json!({
        "user_id": user.id.to_string(),
        "telegram_chat_id": null,
        "code": null,   // будет установлен ботом
        "created_at": Utc::now().timestamp()
    });

    let mut redis = state.redis_conn.clone();
    redis.set_ex(&redis_key, data.to_string(), 900)
        .await
        .map_err(|e| AppError::Redis(e))?;

    let deep_link = format!(
        "https://t.me/{}?start=verify_{}",
        state.config.telegram_bot_username,
        magic_token
    );

    Ok(Json(serde_json::json!({
        "ok": true,
        "data": TelegramResponse {
            deep_link,
            magic_token,
            message: "Перейдите по ссылке в Telegram и введите полученный код сюда".to_string()
        }
    })))
}

// POST /api/user/telegram/verify
pub async fn verify_set_telegram(
    State(state): State<Arc<AppState>>,
    Extension(mut user): Extension<User>,
    Json(payload): Json<VerifyPayload>,
) -> Result<impl IntoResponse, AppError> {
    let redis_key = format!("pending_telegram:{}", payload.magic_token);
    let mut redis = state.redis_conn.clone();

    let data_str: Option<String> = redis.get(&redis_key).await.ok();
    let data_str = data_str.ok_or_else(|| AppError::Validation("Код истёк или недействителен".into()))?;

    let data: serde_json::Value = serde_json::from_str(&data_str)
        .map_err(|_| AppError::Validation("Ошибка данных".into()))?;

    if data["code"].as_str() != Some(&payload.code) {
        return Err(AppError::Validation("Неверный код".into()));
    }

    let telegram_id = data["telegram_chat_id"].as_i64()
        .ok_or(AppError::Validation("Ошибка данных".into()))?;

    let mut conn = state.db_pool.get().map_err(|e| AppError::Pool(e.into()))?;

    diesel::update(users::table.find(user.id))
        .set(users::telegram_id.eq(Some(telegram_id)))
        .execute(&mut conn)?;

    user.telegram_id = Some(telegram_id);

    redis.del::<&str, ()>(&redis_key).await.ok();

    Ok(Json(serde_json::json!({
        "ok": true,
        "data": user
    })))
}

