use axum::{
    extract::Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use axum_extra::extract::cookie::CookieJar;
use serde::Serialize;
use uuid::Uuid;

/// Коды ошибок API
#[derive(Debug, Serialize, Clone)]
pub enum ErrorCode {
    /// Неавторизован/слишком много попыток
    Unauthorized,
    /// Ошибка при взаимодействии с Redis
    Redis,
    /// Ошибка при взаимодействии с БД
    Database,
    /// Срок действия ссылки истёк
    Expired,
    /// Ошибка сериализации (как правило недостижима)
    Serialize,
    /// Ошибка работы с почтой
    Mail,
    /// Ошибка валидации входных данных
    Validation,
    /// Другая ошибка (вспомогательный элемент)
    Other,
}

/// Формат ошибки метода API
#[derive(Debug, Serialize, Clone)]
pub struct ApiError {
    code: ErrorCode,
    message: String,
    details: Option<String>,
}

/// Метаданные ответа API
#[derive(Debug, Serialize, Clone)]
pub struct ApiMeta {
    request_id: Uuid,
}

/// Формат ответа API
#[derive(Debug, Serialize, Clone)]
pub struct ApiResponse<T: Serialize + Clone> {
    ok: bool,
    data: Option<T>,
    meta: ApiMeta,
    error: Option<ApiError>,
}

impl<T: Serialize + Clone> ApiResponse<T> {
    pub fn new_ok(data: T) -> Self {
        Self {
            ok: true,
            data: Some(data),
            meta: ApiMeta {
                request_id: Uuid::new_v4(),
            },
            error: None
        }
    }
}

impl ApiResponse<()> {
    pub fn new_err(
        code: ErrorCode, message: String, details: Option<String>,
    ) -> Self {
        Self {
            ok: false,
            data: None,
            meta: ApiMeta {
                request_id: Uuid::new_v4(),
            },
            error: Some(ApiError {
                code,
                message,
                details,
            })
        }
    }
}

impl<T: Serialize + Clone> IntoResponse for ApiResponse<T> {
    fn into_response(self) -> Response {
        let status_code = if self.ok { StatusCode::OK } else {
            match self.error.clone().unwrap_or(
                ApiError {
                    code: ErrorCode::Other,
                    message: "".to_string(),
                    details: None,
                }
            ).code {
                ErrorCode::Unauthorized => StatusCode::UNAUTHORIZED,
                ErrorCode::Validation | ErrorCode::Expired => StatusCode::BAD_REQUEST,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            }
        };
        (status_code, Json(self)).into_response()
    }
}

/// Вспомогательный тип ответа для методов API
pub type ApiResult<T> = Result<ApiResponse<T>, ApiResponse<()>>;

pub type ApiResultWithCookie<T> = Result<(CookieJar, ApiResponse<T>), ApiResponse<()>>;
