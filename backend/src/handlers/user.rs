use axum::{
    extract::{State, Extension, Multipart},
    Json,
};
use serde::{Deserialize, Serialize};
use chrono::Utc;
use diesel::prelude::*;
use redis::AsyncCommands;

use std::sync::Arc;

use crate::{
    state::AppState,
    models::user::{User, Language, Currency, ApiResponseUser},
    schema::users,
    utils::{mail, token},
    api_response::{ApiResult, ApiResponse, ErrorCode, ApiResponseEmpty},
    handlers::auth::{EmailResponse, ApiResponseEmail},
};

#[derive(utoipa::ToSchema)]
#[derive(Deserialize)]
pub struct UpdateDisplayNamePayload {
    pub display_name: Option<String>,
}

#[derive(utoipa::ToSchema)]
#[derive(Deserialize)]
pub struct UpdateLanguagePayload {
    pub language: Language,
}

#[derive(utoipa::ToSchema)]
#[derive(Deserialize)]
pub struct UpdateCurrencyPayload {
    pub currency: Currency,
}

#[derive(utoipa::ToSchema)]
#[derive(Serialize, Deserialize, Clone)]
pub struct ImageUploadResponse {
    pub success: bool,
    pub link: String,
    pub key: String,
}

type ApiResponseImageUploadResponse = ApiResponse<ImageUploadResponse>;

#[derive(utoipa::ToSchema)]
#[derive(Deserialize)]
pub struct InitiateEmailPayload {
    pub email: String,
}

#[derive(utoipa::ToSchema)]
#[derive(Deserialize)]
pub struct VerifyPayload {
    pub magic_token: String,
    pub code: String,
}

#[derive(utoipa::ToSchema)]
#[derive(Serialize, Clone)]
pub struct TelegramResponse {
    pub deep_link: String,
    pub magic_token: String,
    pub message: String,
}

pub type ApiResponseTelegram = ApiResponse<TelegramResponse>;

/// Получение информации о пользователе (о себе)
#[utoipa::path(
    get,
    path = "/api/v1/user/me",
    tag = "user",
    security(("bearer_token" = [])),
    responses(
        (status = 200, description = "Данные пользователя", body = ApiResponseUser),
        (status = 401, description = "Неавторизован", body = ApiResponseEmpty)
    )
)]
pub async fn get_current_user(
    Extension(user): Extension<User>,
) -> ApiResult<User> {
    Ok(ApiResponse::new_ok(user))
}

/// Изменение имени
#[utoipa::path(
    patch,
    path = "/api/v1/user/display-name",
    tag = "user",
    security(("bearer_token" = [])),
    request_body = UpdateDisplayNamePayload,
    responses(
        (status = 200, description = "Имя обновлено", body = ApiResponseUser),
        (status = 401, description = "Неавторизован", body = ApiResponseEmpty),
        (status = 500, description = "Ошибка сервера", body = ApiResponseEmpty)
    )
)]
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

/// Изменение языка
#[utoipa::path(
    patch,
    path = "/api/v1/user/language",
    tag = "user",
    security(("bearer_token" = [])),
    request_body = UpdateLanguagePayload,
    responses(
        (status = 200, description = "Язык обновлён", body = ApiResponseUser),
        (status = 401, description = "Неавторизован", body = ApiResponseEmpty),
        (status = 500, description = "Ошибка сервера", body = ApiResponseEmpty)
    )
)]
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

/// Изменение валюты
#[utoipa::path(
    patch,
    path = "/api/v1/user/currency",
    tag = "user",
    security(("bearer_token" = [])),
    request_body = UpdateCurrencyPayload,
    responses(
        (status = 200, description = "Валюта обновлена", body = ApiResponseUser),
        (status = 401, description = "Неавторизован", body = ApiResponseEmpty),
        (status = 500, description = "Ошибка сервера", body = ApiResponseEmpty)
    )
)]
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

#[utoipa::path(
    patch,
    path = "/api/v1/user/avatar",
    tag = "user",
    security(("bearer_token" = [])),
    request_body(content_type = "multipart/form-data", description = "Загрузка аватара"),
    responses(
        (status = 200, description = "Аватар успешно обновлён", body = ApiResponseImageUploadResponse),
        (status = 400, description = "Неверный запрос", body = ApiResponseEmpty),
        (status = 401, description = "Неавторизован", body = ApiResponseEmpty),
        (status = 500, description = "Ошибка сервера", body = ApiResponseEmpty)
    )
)]
pub async fn update_avatar(
    State(state): State<Arc<AppState>>,
    Extension(mut user): Extension<User>,
    mut multipart: Multipart,
) -> ApiResult<ImageUploadResponse> {
    let url = format!("{}/upload/image?permanent=true", state.config.storage_service_addr);
    // Первое поле с файлом
    while let Some(field) = multipart.next_field().await.map_err(|e| ApiResponse::new_err(
        ErrorCode::Serialize, "Ошибка получения файла".to_string(), Some(e.to_string())
    ))? {
        if field.file_name().is_none() {
            continue; // Пропуск текстовых полей
        }

        let file_name = field.file_name().unwrap_or("unknown").to_string();
        let content_type = field.content_type().unwrap_or("application/octet-stream").to_string();

        // Побайтовое копирование всего файла в память (Field::bytes читает чанками)
        let file_bytes = field.bytes().await
            .map_err(|e| ApiResponse::new_err(
                ErrorCode::Serialize, "Ошибка чтения файла".to_string(), Some(e.to_string())
            ))?;

        let part = reqwest::multipart::Part::bytes(file_bytes.to_vec())
            .file_name(file_name)
            .mime_str(&content_type)
            .map_err(|e| ApiResponse::new_err(
                ErrorCode::FileService,
                "Ошибка формирования запроса к сервису файлов".to_string(),
                Some(e.to_string()),
            ))?;

        let form = reqwest::multipart::Form::new().part("file", part);

        // Отправляем в файл-сервис
        let response = state
            .http_client
            .post(&url)
            .multipart(form)
            .send()
            .await
            .map_err(|e| ApiResponse::new_err(
                ErrorCode::FileService,
                "Не удалось связаться с сервисом файлов".to_string(),
                Some(e.to_string()),
            ))?;

        if !response.status().is_success() {
            let err_text = response.text().await.unwrap_or_default();
            return Err(ApiResponse::new_err(
                ErrorCode::FileService,
                "Ошибка загрузки файла".to_string(),
                Some(err_text),
            ));
        }

        let storage_resp: ImageUploadResponse = response
            .json()
            .await
            .map_err(|e| ApiResponse::new_err(
                ErrorCode::FileService,
                "Некорректный ответ от файлового сервиса".to_string(),
                Some(e.to_string()),
            ))?;

        // Обновление БД
        let mut conn = state.db_pool.get()
            .map_err(|e| ApiResponse::new_err(
                ErrorCode::Database,
                "Ошибка подключения к БД".to_string(),
                Some(e.to_string()),
            ))?;

        diesel::update(users::table.find(user.id))
            .set((
                users::avatar_url.eq(Some(&storage_resp.link)),
                users::avatar_key.eq(Some(&storage_resp.key)),
            ))
            .execute(&mut conn)
            .map_err(|e| ApiResponse::new_err(
                ErrorCode::Database,
                "Ошибка обновления аватара в БД".to_string(),
                Some(e.to_string()),
            ))?;

        // Если уже есть автар - удаление его
        if let Some(_) = user.avatar_url {
            let _ = state
                .http_client
                .delete(format!(
                    "{}/files/{}",
                    state.config.storage_service_addr,
                    storage_resp.key.clone(),
                ))
            .send()
            .await;
        }

        user.avatar_url = Some(storage_resp.link.clone());
        user.avatar_key = Some(storage_resp.key.clone());

        return Ok(ApiResponse::new_ok(storage_resp));
    }

    Err(ApiResponse::new_err(
        ErrorCode::Serialize, "Файл отсутствует в запросе".to_string(), None
    ))
}

#[utoipa::path(
    delete,
    path = "/api/v1/user/avatar",
    tag = "user",
    security(("bearer_token" = [])),
    responses(
        (status = 200, description = "Аватар успешно удалён", body = ApiResponseUser),
        (status = 400, description = "Неверный запрос", body = ApiResponseEmpty),
        (status = 401, description = "Неавторизован", body = ApiResponseEmpty),
        (status = 500, description = "Ошибка сервера", body = ApiResponseEmpty)
    )
)]
pub async fn delete_avatar(
    State(state): State<Arc<AppState>>,
    Extension(mut user): Extension<User>,
) -> ApiResult<User> {
    // Обновление БД
    let mut conn = state.db_pool.get()
        .map_err(|e| ApiResponse::new_err(
            ErrorCode::Database,
            "Ошибка подключения к БД".to_string(),
            Some(e.to_string()),
        ))?;

    diesel::update(users::table.find(user.id))
        .set((users::avatar_url.eq(None::<String>), users::avatar_key.eq(None::<String>)))
        .execute(&mut conn)
        .map_err(|e| ApiResponse::new_err(
            ErrorCode::Database,
            "Ошибка удаления аватара из БД".to_string(),
            Some(e.to_string()),
        ))?;

    let _ = state
        .http_client
        .delete(format!(
            "{}/files/{}",
            state.config.storage_service_addr,
            user.avatar_key.unwrap_or("".to_string()),
        ))
        .send()
        .await;

    user.avatar_url = None;
    user.avatar_key = None;

    Ok(ApiResponse::new_ok(user))
}

#[utoipa::path(
    patch,
    path = "/api/v1/user/banner",
    tag = "user",
    security(("bearer_token" = [])),
    request_body(content_type = "multipart/form-data", description = "Загрузка аватара"),
    responses(
        (status = 200, description = "Аватар успешно обновлён", body = ApiResponseImageUploadResponse),
        (status = 400, description = "Неверный запрос", body = ApiResponseEmpty),
        (status = 401, description = "Неавторизован", body = ApiResponseEmpty),
        (status = 500, description = "Ошибка сервера", body = ApiResponseEmpty)
    )
)]
pub async fn update_banner(
    State(state): State<Arc<AppState>>,
    Extension(mut user): Extension<User>,
    mut multipart: Multipart,
) -> ApiResult<ImageUploadResponse> {
    let url = format!("{}/upload/image?permanent=true", state.config.storage_service_addr);
    // Первое поле с файлом
    while let Some(field) = multipart.next_field().await.map_err(|e| ApiResponse::new_err(
        ErrorCode::Serialize, "Ошибка получения файла".to_string(), Some(e.to_string())
    ))? {
        if field.file_name().is_none() {
            continue; // Пропуск текстовых полей
        }

        let file_name = field.file_name().unwrap_or("unknown").to_string();
        let content_type = field.content_type().unwrap_or("application/octet-stream").to_string();

        // Побайтовое копирование всего файла в память (Field::bytes читает чанками)
        let file_bytes = field.bytes().await
            .map_err(|e| ApiResponse::new_err(
                ErrorCode::Serialize, "Ошибка чтения файла".to_string(), Some(e.to_string())
            ))?;

        let part = reqwest::multipart::Part::bytes(file_bytes.to_vec())
            .file_name(file_name)
            .mime_str(&content_type)
            .map_err(|e| ApiResponse::new_err(
                ErrorCode::FileService,
                "Ошибка формирования запроса к сервису файлов".to_string(),
                Some(e.to_string()),
            ))?;

        let form = reqwest::multipart::Form::new().part("file", part);

        // Отправляем в файл-сервис
        let response = state
            .http_client
            .post(&url)
            .multipart(form)
            .send()
            .await
            .map_err(|e| ApiResponse::new_err(
                ErrorCode::FileService,
                "Не удалось связаться с сервисом файлов".to_string(),
                Some(e.to_string()),
            ))?;

        if !response.status().is_success() {
            let err_text = response.text().await.unwrap_or_default();
            return Err(ApiResponse::new_err(
                ErrorCode::FileService,
                "Ошибка загрузки файла".to_string(),
                Some(err_text),
            ));
        }

        let storage_resp: ImageUploadResponse = response
            .json()
            .await
            .map_err(|e| ApiResponse::new_err(
                ErrorCode::FileService,
                "Некорректный ответ от файлового сервиса".to_string(),
                Some(e.to_string()),
            ))?;

        // Обновление БД
        let mut conn = state.db_pool.get()
            .map_err(|e| ApiResponse::new_err(
                ErrorCode::Database,
                "Ошибка подключения к БД".to_string(),
                Some(e.to_string()),
            ))?;

        diesel::update(users::table.find(user.id))
            .set((
                users::banner_url.eq(Some(&storage_resp.link)),
                users::banner_key.eq(Some(&storage_resp.key)),
            ))
            .execute(&mut conn)
            .map_err(|e| ApiResponse::new_err(
                ErrorCode::Database,
                "Ошибка обновления аватара в БД".to_string(),
                Some(e.to_string()),
            ))?;

        // Если уже есть баннер - удаление его
        if let Some(_) = user.banner_url {
            let _ = state
                .http_client
                .delete(format!(
                    "{}/files/{}",
                    state.config.storage_service_addr,
                    storage_resp.key.clone(),
                ))
            .send()
            .await;
        }

        user.banner_url = Some(storage_resp.link.clone());
        user.banner_key = Some(storage_resp.key.clone());

        return Ok(ApiResponse::new_ok(storage_resp));
    }

    Err(ApiResponse::new_err(
        ErrorCode::Serialize, "Файл отсутствует в запросе".to_string(), None
    ))
}

#[utoipa::path(
    delete,
    path = "/api/v1/user/banner",
    tag = "user",
    security(("bearer_token" = [])),
    responses(
        (status = 200, description = "Баннер успешно удалён", body = ApiResponseUser),
        (status = 400, description = "Неверный запрос", body = ApiResponseEmpty),
        (status = 401, description = "Неавторизован", body = ApiResponseEmpty),
        (status = 500, description = "Ошибка сервера", body = ApiResponseEmpty)
    )
)]
pub async fn delete_banner(
    State(state): State<Arc<AppState>>,
    Extension(mut user): Extension<User>,
) -> ApiResult<User> {
    // Обновление БД
    let mut conn = state.db_pool.get()
        .map_err(|e| ApiResponse::new_err(
            ErrorCode::Database,
            "Ошибка подключения к БД".to_string(),
            Some(e.to_string()),
        ))?;

    diesel::update(users::table.find(user.id))
        .set((users::banner_url.eq(None::<String>), users::banner_key.eq(None::<String>)))
        .execute(&mut conn)
        .map_err(|e| ApiResponse::new_err(
            ErrorCode::Database,
            "Ошибка удаления баннера из БД".to_string(),
            Some(e.to_string()),
        ))?;

    let _ = state
        .http_client
        .delete(format!(
            "{}/files/{}",
            state.config.storage_service_addr,
            user.banner_key.unwrap_or("".to_string()),
        ))
        .send()
        .await;

    user.banner_url = None;
    user.banner_key = None;

    Ok(ApiResponse::new_ok(user))
}

/// Отправка кода для установки email
#[utoipa::path(
    post,
    path = "/api/v1/user/email",
    tag = "user",
    security(("bearer_token" = [])),
    request_body = InitiateEmailPayload,
    responses(
        (status = 200, description = "Код отправлен", body = ApiResponseEmail),
        (status = 400, description = "Email уже установлен", body = ApiResponseEmpty),
        (status = 401, description = "Неавторизован", body = ApiResponseEmpty),
        (status = 500, description = "Ошибка сервера", body = ApiResponseEmpty)
    )
)]
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

/// Подтверждение установки email
#[utoipa::path(
    post,
    path = "/api/v1/user/email/verify",
    tag = "user",
    security(("bearer_token" = [])),
    request_body = VerifyPayload,
    responses(
        (status = 200, description = "Email подтверждён", body = ApiResponseUser),
        (status = 400, description = "Код истёк или недействителен", body = ApiResponseEmpty),
        (status = 401, description = "Неавторизован", body = ApiResponseEmpty),
        (status = 500, description = "Ошибка сервера", body = ApiResponseEmpty)
    )
)]
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

/// Отправка кода для установки telegram
#[utoipa::path(
    post,
    path = "/api/v1/user/telegram",
    tag = "user",
    security(("bearer_token" = [])),
    responses(
        (status = 200, description = "Deep link сгенерирован", body = ApiResponseTelegram),
        (status = 400, description = "Email уже установлен", body = ApiResponseEmpty),
        (status = 401, description = "Неавторизован", body = ApiResponseEmpty),
        (status = 500, description = "Ошибка сервера", body = ApiResponseEmpty)
    )
)]
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

/// Подтверждение установки telegram
#[utoipa::path(
    post,
    path = "/api/v1/user/telegram/verify",
    tag = "user",
    security(("bearer_token" = [])),
    request_body = VerifyPayload,
    responses(
        (status = 200, description = "Telegram привязан", body = ApiResponseUser),
        (status = 400, description = "Код истёк или недействителен", body = ApiResponseEmpty),
        (status = 401, description = "Неавторизован", body = ApiResponseEmpty),
        (status = 500, description = "Ошибка сервера", body = ApiResponseEmpty)
    )
)]
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

