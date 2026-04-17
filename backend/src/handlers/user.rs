use axum::{
    extract::{State, Extension},
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
    utils::{mail, token},
    api_response::{ApiResult, ApiResponse, ErrorCode},
    handlers::auth::EmailResponse,
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

#[derive(Serialize, Clone)]
pub struct TelegramResponse {
    pub deep_link: String,
    pub magic_token: String,
    pub message: String,
}

// GET /api/user/me
pub async fn get_current_user(
    Extension(user): Extension<User>,
) -> ApiResult<User> {
    Ok(ApiResponse::new_ok(user))
}

// PATCH /api/v1/user/display-name
pub async fn update_display_name(
    State(state): State<Arc<AppState>>,
    Extension(mut user): Extension<User>,
    Json(payload): Json<UpdateDisplayNamePayload>,
) -> ApiResult<User> {
    let mut conn = state.db_pool.get()
        .map_err(|e| ApiResponse::new_err(
            ErrorCode::Database,
            "Ошибка при подключении к БД, попробуйте позже".to_string(),
            Some(e.to_string()),
        ))?;

    diesel::update(users::table.find(user.id))
        .set(users::display_name.eq(&payload.display_name))
        .execute(&mut conn)
        .map_err(|e| ApiResponse::new_err(
            ErrorCode::Database,
            "Ошибка при работе с БД, попробуйте позже".to_string(),
            Some(e.to_string()),
        ))?;

    user.display_name = payload.display_name;

    Ok(ApiResponse::new_ok(user))
}

// PATCH /api/v1/user/language
pub async fn update_language(
    State(state): State<Arc<AppState>>,
    Extension(mut user): Extension<User>,
    Json(payload): Json<UpdateLanguagePayload>,
) -> ApiResult<User> {
    let mut conn = state.db_pool.get()
        .map_err(|e| ApiResponse::new_err(
            ErrorCode::Database,
            "Ошибка при подключении к БД, попробуйте позже".to_string(),
            Some(e.to_string()),
        ))?;

    diesel::update(users::table.find(user.id))
        .set(users::language.eq(&payload.language))
        .execute(&mut conn)
        .map_err(|e| ApiResponse::new_err(
            ErrorCode::Database,
            "Ошибка при работе с БД, попробуйте позже".to_string(),
            Some(e.to_string()),
        ))?;

    user.language = payload.language;

    Ok(ApiResponse::new_ok(user))
}

// PATCH /api/v1/user/currency
pub async fn update_currency(
    State(state): State<Arc<AppState>>,
    Extension(mut user): Extension<User>,
    Json(payload): Json<UpdateCurrencyPayload>,
) -> ApiResult<User> {
    let mut conn = state.db_pool.get()
        .map_err(|e| ApiResponse::new_err(
            ErrorCode::Database,
            "Ошибка при подключении к БД, попробуйте позже".to_string(),
            Some(e.to_string()),
        ))?;

    diesel::update(users::table.find(user.id))
        .set(users::currency.eq(&payload.currency))
        .execute(&mut conn)
        .map_err(|e| ApiResponse::new_err(
            ErrorCode::Database,
            "Ошибка при работе с БД, попробуйте позже".to_string(),
            Some(e.to_string()),
        ))?;

    user.currency = payload.currency;

    Ok(ApiResponse::new_ok(user))
}

// POST /api/v1/user/email
pub async fn initiate_set_email(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    Json(payload): Json<InitiateEmailPayload>,
) -> ApiResult<EmailResponse> {
    if user.email.is_some() {
        return Err(ApiResponse::new_err(
            ErrorCode::Validation,
            "Email уже установлен".to_string(),
            None,
        ));
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
    redis.set_ex::<_, _, ()>(&redis_key, data.to_string(), 900)
        .await
        .map_err(|e| ApiResponse::new_err(
            ErrorCode::Redis,
            "Неизвестная ошибка, повторите запрос позже".to_string(),
            Some(format!("Redis: {}", e)),
        ))?;

    mail::send_mail(
        &state.config.smtp_host,
        state.config.smtp_port,
        &state.config.email_addr,
        &payload.email,
        "Подтверждение email",
        format!("Код подтверждения: {}", code),
    ).await
    .map_err(|e| ApiResponse::new_err(
        ErrorCode::Mail,
        "Не удалось отправить письмо".to_string(),
        Some(e),
    ))?;

    Ok(ApiResponse::new_ok(EmailResponse {
        magic_token,
        message: "Код отправлен на ваш email".to_string()
    }))
}

// POST /api/v1/user/email/verify
pub async fn verify_set_email(
    State(state): State<Arc<AppState>>,
    Extension(mut user): Extension<User>,
    Json(payload): Json<VerifyPayload>,
) -> ApiResult<User> {
    let redis_key = format!("pending_email:{}", payload.magic_token);
    let mut redis = state.redis_conn.clone();

    let data_str: Option<String> = redis.get(&redis_key).await.ok();
    let data_str = data_str.ok_or_else(|| ApiResponse::new_err(
        ErrorCode::Expired,
        "Код истёк или недействителен".to_string(),
        None,
    ))?;

    let data: serde_json::Value = serde_json::from_str(&data_str)
        .map_err(|e| ApiResponse::new_err(
            ErrorCode::Serialize,
            "Ошибка сериализации данных, попробуйте снова".to_string(),
            Some(e.to_string()),
        ))?;

    if data["code"].as_str() != Some(&payload.code) {
        return Err(ApiResponse::new_err(
            ErrorCode::Validation,
            "Неверный код".to_string(),
            None,
        ));
    }

    let email = data["email"].as_str()
        .ok_or(ApiResponse::new_err(
            ErrorCode::Validation,
            "Ошибка данных".to_string(),
            None,
        ))?
        .to_string();

    let mut conn = state.db_pool.get()
        .map_err(|e| ApiResponse::new_err(
            ErrorCode::Database,
            "Ошибка при подключении к БД, попробуйте позже".to_string(),
            Some(e.to_string()),
        ))?;

    diesel::update(users::table.find(user.id))
        .set(users::email.eq(Some(email.clone())))
        .execute(&mut conn)
        .map_err(|e| ApiResponse::new_err(
            ErrorCode::Database,
            "Ошибка при работе с БД, попробуйте позже".to_string(),
            Some(e.to_string()),
        ))?;

    user.email = Some(email);

    redis.del::<&str, ()>(&redis_key).await.ok();

    Ok(ApiResponse::new_ok(user))
}

// POST /api/v1/user/telegram
pub async fn initiate_set_telegram(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
) -> ApiResult<TelegramResponse> {
    if user.telegram_id.is_some() {
        return Err(ApiResponse::new_err(
            ErrorCode::Validation,
            "Telegram уже установлен".to_string(),
            None,
        ));
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
    redis.set_ex::<_, _, ()>(&redis_key, data.to_string(), 900)
        .await
        .map_err(|e| ApiResponse::new_err(
            ErrorCode::Redis,
            "Неизвестная ошибка, повторите запрос позже".to_string(),
            Some(format!("Redis: {}", e)),
        ))?;

    let deep_link = format!(
        "https://t.me/{}?start=verify_{}",
        state.config.telegram_bot_username,
        magic_token
    );

    Ok(ApiResponse::new_ok(TelegramResponse {
        deep_link,
        magic_token,
        message: "Перейдите по ссылке в Telegram и введите полученный код сюда".to_string()
    }))
}

// POST /api/v1/user/telegram/verify
pub async fn verify_set_telegram(
    State(state): State<Arc<AppState>>,
    Extension(mut user): Extension<User>,
    Json(payload): Json<VerifyPayload>,
) -> ApiResult<User> {
    let redis_key = format!("pending_telegram:{}", payload.magic_token);
    let mut redis = state.redis_conn.clone();

    let data_str: Option<String> = redis.get(&redis_key).await.ok();
    let data_str = data_str.ok_or_else(|| ApiResponse::new_err(
        ErrorCode::Expired,
        "Код истёк или недействителен".to_string(),
        None,
    ))?;

    let data: serde_json::Value = serde_json::from_str(&data_str)
        .map_err(|e| ApiResponse::new_err(
            ErrorCode::Serialize,
            "Ошибка сериализации данных, попробуйте снова".to_string(),
            Some(e.to_string()),
        ))?;

    if data["code"].as_str() != Some(&payload.code) {
        return Err(ApiResponse::new_err(
            ErrorCode::Validation,
            "Неверный код".to_string(),
            None,
        ));
    }

    let telegram_id = data["telegram_chat_id"].as_i64()
        .ok_or(ApiResponse::new_err(
            ErrorCode::Validation,
            "Ошибка данных".to_string(),
            None,
        ))?;

    let mut conn = state.db_pool.get()
        .map_err(|e| ApiResponse::new_err(
            ErrorCode::Database,
            "Ошибка при подключении к БД, попробуйте позже".to_string(),
            Some(e.to_string()),
        ))?;

    diesel::update(users::table.find(user.id))
        .set(users::telegram_id.eq(Some(telegram_id)))
        .execute(&mut conn)
        .map_err(|e| ApiResponse::new_err(
            ErrorCode::Database,
            "Ошибка при работе с БД, попробуйте позже".to_string(),
            Some(e.to_string()),
        ))?;

    user.telegram_id = Some(telegram_id);

    redis.del::<&str, ()>(&redis_key).await.ok();

    Ok(ApiResponse::new_ok(user))
}

