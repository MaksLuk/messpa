use axum::{
    extract::{State, Extension, ConnectInfo},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use axum_extra::extract::cookie::{CookieJar, Cookie};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{Utc, Duration};
use diesel::prelude::*;
use redis::AsyncCommands;

use std::sync::Arc;
use std::net::SocketAddr;

use crate::{
    state::AppState,
    models::user::{User, NewUser, RefreshSession, NewRefreshSession},
    schema::{users, refresh_sessions},
    error::AppError,
    utils::{jwt, token, rate_limit, telegram, mail},
};

#[derive(Serialize)]
pub struct SendCodeResponse {
    pub deep_link: String,
    pub magic_token: String,
    pub message: String,
}

#[derive(Deserialize)]
pub struct VerifyPayload {
    pub magic_token: String,
    pub code: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub access_token: String,
    pub user: User,
}

#[derive(Serialize)]
pub struct RefreshResponse {
    pub access_token: String,
}

#[derive(Serialize)]
pub struct EmailResponse {
    pub magic_token: String,
    pub message: String,
}

#[derive(Deserialize)]
pub struct SendEmailPayload {
    pub email: String,
}

/// POST /api/auth/telegram/send-code
pub async fn send_telegram_code(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Result<impl IntoResponse, AppError> {
    let ip = addr.ip();
    let mut redis = state.redis_conn.clone();

    let ip_key = format!("rate:login:init:ip:{}", ip);
    if !rate_limit::check_and_increment(&mut redis, &ip_key, 6, 900).await.map_err(AppError::Redis)? {
        return Err(AppError::Auth("Слишком много попыток. Подождите.".into()));
    }

    let magic_token = token::generate_magic_token();
    let redis_key = format!("pending_auth:{}", magic_token);
    let data = serde_json::json!({
        "ip": ip.to_string(),
        "created_at": Utc::now().timestamp(),
        "code": null,           // код будет установлен ботом
        "telegram_chat_id": null,
        "email": null
    });

    redis.set_ex(&redis_key, data.to_string(), 900) // 15 минут
        .await
        .map_err(|e| AppError::Auth(format!("Redis: {}", e)))?;

    let deep_link = format!(
        "https://t.me/{}?start=auth_{}",
        state.config.telegram_bot_username,
        magic_token
    );

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "ok": true,
            "data": SendCodeResponse {
                deep_link,
                magic_token,
                message: "Перейдите по ссылке в Telegram и введите полученный код сюда".to_string()
            }
        }))
    ))
}

/// POST /api/auth/telegram/verify
pub async fn verify_telegram_code(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    Json(payload): Json<VerifyPayload>,
) -> Result<impl IntoResponse, AppError> {
    let mut redis = state.redis_conn.clone();

    let redis_key = format!("pending_auth:{}", payload.magic_token);
    let data_str: Option<String> = redis.get(&redis_key).await.ok();
    let Some(data_str) = data_str else {
        return Err(AppError::Auth("Ссылка истекла или недействительна".into()));
    };

    let data: serde_json::Value = serde_json::from_str(&data_str)
        .map_err(|_| AppError::Auth("Ошибка данных".into()))?;

    let stored_code = data["code"].as_str()
        .ok_or_else(|| AppError::Auth("Код ещё не был отправлен ботом".into()))?;

    if stored_code != payload.code {
        return Err(AppError::Auth("Неверный код".into()));
    }

    let telegram_chat_id = data["telegram_chat_id"].as_i64()
        .ok_or_else(|| AppError::Auth("Данные от бота не получены".into()))?;

    // Удаляем использованную запись
    redis.del::<&str, ()>(&redis_key).await.ok();

    let mut conn = state.db_pool.get()
        .map_err(|e| AppError::Pool(e.into()))?;

    let user: User = match users::table
        .filter(users::telegram_id.eq(Some(telegram_chat_id)))
        .select(User::as_select())
        .first::<User>(&mut conn)
        .optional()?
    {
        Some(u) => u,
        None => {
            let new_user = NewUser {
                email: None,
                role: crate::models::user::UserRole::Client,
                telegram_id: Some(telegram_chat_id),
                display_name: None,
                avatar_url: None,
                banner_url: None,
                description: None,
                language: crate::models::user::Language::Ru,
                currency: crate::models::user::Currency::Rub,
                is_executor: Some(false),
            };
            diesel::insert_into(users::table)
                .values(&new_user)
                .get_result::<User>(&mut conn)?
        }
    };

    // Создаём сессию
    let family_id = Uuid::new_v4();
    let refresh_token = token::create_refresh_token(family_id);
    let (_, random_part) = token::parse_refresh_token(&refresh_token)
        .ok_or_else(|| AppError::Auth("Ошибка токена".into()))?;

    let token_hash = token::hash_token(&random_part, &state.config.refresh_pepper);

    let new_session = NewRefreshSession {
        user_id: user.id,
        family_id,
        token_hash,
        device_fingerprint: "telegram-code-flow".to_string(),
        ip_address: "0.0.0.0".parse().unwrap(),
        user_agent: "telegram-bot".to_string(),
        expires_at: (Utc::now() + Duration::days(state.config.jwt_refresh_ttl)).naive_utc(),
    };

    diesel::insert_into(refresh_sessions::table)
        .values(&new_session)
        .execute(&mut conn)?;

    let access_token = jwt::generate_access_token(
        user.id,
        &state.config.jwt_secret,
    );

    let cookie = Cookie::build(("refresh_token", refresh_token))
        .http_only(true)
        .secure(true)
        .same_site(axum_extra::extract::cookie::SameSite::Strict)
        .path("/")
        .max_age(time::Duration::days(state.config.jwt_refresh_ttl as i64))
        .build();

    let response = (
        StatusCode::OK,
        Json(serde_json::json!({
            "ok": true,
            "data": AuthResponse { access_token, user }
        }))
    ).into_response();

    let jar = jar.add(cookie);
    Ok((jar, response))
}

/// POST /api/auth/email/send-code
pub async fn send_email_code(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(payload): Json<SendEmailPayload>,
) -> Result<impl IntoResponse, AppError> {
    let ip = addr.ip();
    let mut redis = state.redis_conn.clone();

    let ip_key = format!("rate:login:init:ip:{}", ip);
    if !rate_limit::check_and_increment(&mut redis, &ip_key, 6, 900).await.map_err(AppError::Redis)? {
        return Err(AppError::Auth("Слишком много попыток. Подождите.".into()));
    }

    let magic_token = token::generate_magic_token();
    let redis_key = format!("pending_auth:{}", magic_token);
    let code: String = format!("{:06}", rand::random::<u32>() % 1_000_000);
    let data = serde_json::json!({
        "ip": ip.to_string(),
        "created_at": Utc::now().timestamp(),
        "code": code.clone(),
        "telegram_chat_id": null,
        "email": payload.email
    });

    redis.set_ex(&redis_key, data.to_string(), 900) // 15 минут
        .await
        .map_err(|e| AppError::Auth(format!("Redis: {}", e)))?;

    mail::send_mail(
        &state.config.smtp_host,
        state.config.smtp_port,
        &state.config.email_addr,
        &payload.email,
        "Подтверждение email",
        format!("Код подтверждения: {}", code),
    ).await
    .map_err(|e| AppError::Auth(format!("Не удалось отправить письмо: {}", e)))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "ok": true,
            "data": EmailResponse {
                magic_token,
                message: "Код отправлен на ваш email".to_string()
            }
        }))
    ))
}

// POST /api/auth/email/verify
pub async fn verify_email_code(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    Json(payload): Json<VerifyPayload>,
) -> Result<impl IntoResponse, AppError> {
    let mut redis = state.redis_conn.clone();

    let redis_key = format!("pending_auth:{}", payload.magic_token);
    let data_str: Option<String> = redis.get(&redis_key).await.ok();
    let Some(data_str) = data_str else {
        return Err(AppError::Auth("Ссылка истекла или недействительна".into()));
    };

    let data: serde_json::Value = serde_json::from_str(&data_str)
        .map_err(|_| AppError::Auth("Ошибка данных".into()))?;

    let stored_code = data["code"].as_str()
        .ok_or_else(|| AppError::Auth("Код ещё не был отправлен".into()))?; // ошибка недостижима

    if stored_code != payload.code {
        return Err(AppError::Auth("Неверный код".into()));
    }

    let email = data["email"].as_str()
        .ok_or_else(|| AppError::Auth("Данные не получены".into()))?;

    // Удаляем использованную запись
    redis.del::<&str, ()>(&redis_key).await.ok();

    let mut conn = state.db_pool.get()
        .map_err(|e| AppError::Pool(e.into()))?;

    let user: User = match users::table
        .filter(users::email.eq(Some(email.clone())))
        .select(User::as_select())
        .first::<User>(&mut conn)
        .optional()?
    {
        Some(u) => u,
        None => {
            let new_user = NewUser {
                email: Some(email.to_string()),
                role: crate::models::user::UserRole::Client,
                telegram_id: None,
                display_name: None,
                avatar_url: None,
                banner_url: None,
                description: None,
                language: crate::models::user::Language::Ru,
                currency: crate::models::user::Currency::Rub,
                is_executor: Some(false),
            };
            diesel::insert_into(users::table)
                .values(&new_user)
                .get_result::<User>(&mut conn)?
        }
    };

    // Создаём сессию
    let family_id = Uuid::new_v4();
    let refresh_token = token::create_refresh_token(family_id);
    let (_, random_part) = token::parse_refresh_token(&refresh_token)
        .ok_or_else(|| AppError::Auth("Ошибка токена".into()))?;

    let token_hash = token::hash_token(&random_part, &state.config.refresh_pepper);

    let new_session = NewRefreshSession {
        user_id: user.id,
        family_id,
        token_hash,
        device_fingerprint: "email-code-flow".to_string(),
        ip_address: "0.0.0.0".parse().unwrap(),
        user_agent: "email".to_string(),
        expires_at: (Utc::now() + Duration::days(state.config.jwt_refresh_ttl)).naive_utc(),
    };

    diesel::insert_into(refresh_sessions::table)
        .values(&new_session)
        .execute(&mut conn)?;

    let access_token = jwt::generate_access_token(
        user.id,
        &state.config.jwt_secret,
    );

    let cookie = Cookie::build(("refresh_token", refresh_token))
        .http_only(true)
        .secure(true)
        .same_site(axum_extra::extract::cookie::SameSite::Strict)
        .path("/")
        .max_age(time::Duration::days(state.config.jwt_refresh_ttl as i64))
        .build();

    let response = (
        StatusCode::OK,
        Json(serde_json::json!({
            "ok": true,
            "data": AuthResponse { access_token, user }
        }))
    ).into_response();

    let jar = jar.add(cookie);
    Ok((jar, response))
}

/// POST /api/auth/refresh
pub async fn refresh_token(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
) -> Result<impl IntoResponse, AppError> {
    let refresh_token = jar
        .get("refresh_token")
        .map(|c| c.value().to_string())
        .ok_or_else(|| AppError::Auth("Refresh token отсутствует".into()))?;

    let (family_id, random_part) = token::parse_refresh_token(&refresh_token)
        .ok_or_else(|| AppError::Auth("Неверный формат refresh token".into()))?;

    let token_hash = token::hash_token(&random_part, &state.config.refresh_pepper);

    let mut conn = state.db_pool.get()
        .map_err(|e| AppError::Pool(e.into()))?;

    let session: RefreshSession = refresh_sessions::table
        .filter(refresh_sessions::family_id.eq(family_id))
        .filter(refresh_sessions::revoked.eq(false))
        .filter(refresh_sessions::expires_at.gt(Utc::now().naive_utc()))
        .first::<RefreshSession>(&mut conn)
        .optional()?
        .ok_or_else(|| AppError::Auth("Refresh token недействителен или истёк".into()))?;

    if session.token_hash != token_hash {
        // Обнаружен reuse → инвалидируем всю семью
        diesel::update(refresh_sessions::table.filter(refresh_sessions::family_id.eq(family_id)))
            .set(refresh_sessions::revoked.eq(true))
            .execute(&mut conn)?;

        return Err(AppError::Auth("Повторное использование refresh token. Все сессии сброшены.".into()));
    }

    // Rotation токена
    let new_refresh_token = token::create_refresh_token(family_id);
    let (_, new_random_part) = token::parse_refresh_token(&new_refresh_token).unwrap();
    let new_hash = token::hash_token(&new_random_part, &state.config.refresh_pepper);

    diesel::update(refresh_sessions::table.find(session.id))
        .set((
            refresh_sessions::token_hash.eq(new_hash),
            refresh_sessions::expires_at.eq((Utc::now() + Duration::days(state.config.jwt_refresh_ttl)).naive_utc()),
        ))
        .execute(&mut conn)?;

    let new_access_token = jwt::generate_access_token(
        session.user_id,
        &state.config.jwt_secret,
    );

    let cookie = Cookie::build(("refresh_token", new_refresh_token))
        .http_only(true)
        .secure(if state.config.test { false } else { true })
        .same_site(if state.config.test {
            axum_extra::extract::cookie::SameSite::Lax
        } else {
            axum_extra::extract::cookie::SameSite::Strict
        })
        .path("/")
        .max_age(time::Duration::days(state.config.jwt_refresh_ttl as i64))
        .build();

    let jar = jar.add(cookie);

    Ok((
        jar,
        Json(serde_json::json!({
            "ok": true,
            "data": RefreshResponse { access_token: new_access_token }
        }))
    ))
}

/// POST /api/auth/logout
pub async fn logout(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
) -> Result<impl IntoResponse, AppError> {
    let refresh_token = jar
        .get("refresh_token")
        .map(|c| c.value().to_string())
        .ok_or_else(|| AppError::Auth("Нет активной сессии".into()))?;

    let (family_id, _) = token::parse_refresh_token(&refresh_token)
        .ok_or_else(|| AppError::Auth("Неверный токен".into()))?;

    let mut conn = state.db_pool.get()
        .map_err(|e| AppError::Pool(e.into()))?;

    diesel::update(refresh_sessions::table.filter(refresh_sessions::family_id.eq(family_id)))
        .set(refresh_sessions::revoked.eq(true))
        .execute(&mut conn)?;

    let expired_cookie = Cookie::build(("refresh_token", ""))
        .max_age(time::Duration::seconds(0))
        .path("/")
        .build();

    let jar = jar.add(expired_cookie);

    Ok((
        jar,
        Json(serde_json::json!({"ok": true, "message": "Вы успешно вышли из системы"}))
    ))
}

/// POST /api/auth/logout-all
pub async fn logout_all(
    State(state): State<Arc<AppState>>,
    Extension(user_id): Extension<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let mut conn = state.db_pool.get()
        .map_err(|e| AppError::Pool(e.into()))?;

    diesel::update(refresh_sessions::table.filter(refresh_sessions::user_id.eq(user_id)))
        .set(refresh_sessions::revoked.eq(true))
        .execute(&mut conn)?;

    Ok(Json(serde_json::json!({
        "ok": true,
        "message": "Все сессии завершены"
    })))
}

