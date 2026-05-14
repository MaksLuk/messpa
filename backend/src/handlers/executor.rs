use axum::{
    extract::{State, Extension},
    Json,
};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use serde_json::Value as JsonValue;

use std::sync::Arc;

use crate::{
    state::AppState,
    models::user::{User, UserInfoExecutor, ApiResponseExecutor},
    schema::{users, user_info_executor, specializations},
    api_response::{ApiResult, ApiResponse, ErrorCode, ApiResponseEmpty},
};

/// Становление исполнителем
#[utoipa::path(
    post,
    path = "/api/v1/user/executor",
    tag = "executor",
    security(("bearer_token" = [])),
    responses(
        (status = 200, description = "Пользователь успешно стал исполнителем", body = ApiResponseExecutor),
        (status = 401, description = "Неавторизован", body = ApiResponseEmpty),
        (status = 500, description = "Ошибка сервера", body = ApiResponseEmpty)
    )
)]
pub async fn become_executor(
    State(state): State<Arc<AppState>>,
    Extension(mut user): Extension<User>,
) -> ApiResult<UserInfoExecutor> {
    let mut conn = state.db_pool.get()
        .map_err(|e| ApiResponse::new_err(
            ErrorCode::Database,
            "Ошибка при подключении к БД, попробуйте позже".to_string(),
            Some(e.to_string()),
        ))?;

    // Изменение параметра пользователя на "исполнитель"
    diesel::update(users::table.find(user.id))
        .set(users::is_executor.eq(true))
        .execute(&mut conn)
        .map_err(|e| ApiResponse::new_err(
            ErrorCode::Database,
            "Ошибка при работе с БД, попробуйте позже".to_string(),
            Some(e.to_string()),
        ))?;

    user.is_executor = true;

    // Добавление (если нет) и получение информации о исполнителе
    let executor_info: UserInfoExecutor = match user_info_executor::table
        .filter(user_info_executor::user_id.eq(user.id))
        .select(UserInfoExecutor::as_select())
        .first::<UserInfoExecutor>(&mut conn)
        .optional()
        .map_err(|e| ApiResponse::new_err(
            ErrorCode::Database,
            "Ошибка при работе с БД, попробуйте позже".to_string(),
            Some(e.to_string()),
        ))?
    {
        Some(u) => u,
        None => {
            let new_executor_info = UserInfoExecutor {
                user_id: user.id.clone(),
                specialization: None,
                rating: None,
                review_count: None,
                completed_orders: None,
                timezone: None,
                work_schedule: None,
                contact_rules: None,
            };
            diesel::insert_into(user_info_executor::table)
                .values(&new_executor_info)
                .get_result::<UserInfoExecutor>(&mut conn)
                .map_err(|e| ApiResponse::new_err(
                    ErrorCode::Database,
                    "Ошибка при работе с БД, попробуйте позже".to_string(),
                    Some(e.to_string()),
                ))?
        }
    };

    Ok(ApiResponse::new_ok(executor_info))
}

/// Прекращение роли исполнителя
#[utoipa::path(
    delete,
    path = "/api/v1/user/executor",
    tag = "executor",
    security(("bearer_token" = [])),
    responses(
        (status = 200, description = "Пользователь успешно перестал быть исполнителем", body = ApiResponseEmpty),
        (status = 401, description = "Неавторизован", body = ApiResponseEmpty),
        (status = 500, description = "Ошибка сервера", body = ApiResponseEmpty)
    )
)]
pub async fn stop_beeng_executor(
    State(state): State<Arc<AppState>>,
    Extension(mut user): Extension<User>,
) -> ApiResult<()> {
    let mut conn = state.db_pool.get()
        .map_err(|e| ApiResponse::new_err(
            ErrorCode::Database,
            "Ошибка при подключении к БД, попробуйте позже".to_string(),
            Some(e.to_string()),
        ))?;

    // Изменение роли пользователя на "исполнитель"
    diesel::update(users::table.find(user.id))
        .set(users::is_executor.eq(false))
        .execute(&mut conn)
        .map_err(|e| ApiResponse::new_err(
            ErrorCode::Database,
            "Ошибка при работе с БД, попробуйте позже".to_string(),
            Some(e.to_string()),
        ))?;

    user.is_executor = false;
    Ok(ApiResponse::new_ok(()))
}

/// Получение информации о себе как об исполнителе
#[utoipa::path(
    get,
    path = "/api/v1/user/executor/me",
    tag = "executor",
    security(("bearer_token" = [])),
    responses(
        (status = 200, description = "Информация о себе", body = ApiResponseExecutor),
        (status = 401, description = "Неавторизован", body = ApiResponseEmpty),
        (status = 500, description = "Ошибка сервера", body = ApiResponseEmpty)
    )
)]
pub async fn get_user_executor(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
) -> ApiResult<UserInfoExecutor> {
    let mut conn = state.db_pool.get()
        .map_err(|e| ApiResponse::new_err(
            ErrorCode::Database,
            "Ошибка при подключении к БД, попробуйте позже".to_string(),
            Some(e.to_string()),
        ))?;

    let user: UserInfoExecutor = user_info_executor::table
        .filter(user_info_executor::user_id.eq(user.id))
        .select(UserInfoExecutor::as_select())
        .first::<UserInfoExecutor>(&mut conn)
        .map_err(|e| ApiResponse::new_err(
            ErrorCode::Database,
            "Ошибка при работе с БД, попробуйте позже".to_string(),
            Some(e.to_string()),
        ))?;

    Ok(ApiResponse::new_ok(user))
}

#[derive(ToSchema, Deserialize)]
pub struct UpdateSpecializationRequest {
    pub specialization_id: i32,
}

#[derive(ToSchema, Deserialize)]
pub struct UpdateTimezoneRequest {
    pub timezone: String,
}

#[derive(ToSchema, Deserialize, Serialize, Clone, Debug)]
pub struct DailySchedule {
    pub start: String, // формат "HH:MM"
    pub end: String,   // формат "HH:MM"
}

#[derive(ToSchema, Deserialize, Serialize, Clone, Debug)]
pub struct UpdateWorkScheduleRequest {
    pub monday: Option<DailySchedule>,
    pub tuesday: Option<DailySchedule>,
    pub wednesday: Option<DailySchedule>,
    pub thursday: Option<DailySchedule>,
    pub friday: Option<DailySchedule>,
    pub saturday: Option<DailySchedule>,
    pub sunday: Option<DailySchedule>,
}

#[derive(ToSchema, Deserialize, Serialize, Clone, Debug)]
pub struct ContactInfo {
    pub name: String,
    pub value: String,
}

#[derive(ToSchema, Deserialize)]
pub struct UpdateContactRulesRequest {
    pub contacts: Vec<ContactInfo>,
}

/// Изменение специализации исполнителя (с проверкой существования ID в таблице specializations)
#[utoipa::path(
    patch,
    path = "/api/v1/user/executor/specialization",
    tag = "executor",
    security(("bearer_token" = [])),
    request_body = UpdateSpecializationRequest,
    responses(
        (status = 200, description = "Специализация обновлена", body = ApiResponseExecutor),
        (status = 400, description = "Некорректный запрос", body = ApiResponseEmpty),
        (status = 401, description = "Неавторизован", body = ApiResponseEmpty),
        (status = 404, description = "Специализация не найдена", body = ApiResponseEmpty),
        (status = 500, description = "Ошибка сервера", body = ApiResponseEmpty)
    )
)]
pub async fn update_specialization(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    Json(payload): Json<UpdateSpecializationRequest>,
) -> ApiResult<UserInfoExecutor> {
    let mut conn = state.db_pool.get()
        .map_err(|e| ApiResponse::new_err(
            ErrorCode::Database,
            "Ошибка при подключении к БД, попробуйте позже".to_string(),
            Some(e.to_string()),
        ))?;

    // Проверка существования специализации
    let spec_exists: bool = diesel::select(diesel::dsl::exists(
        specializations::table.find(payload.specialization_id)
    ))
    .get_result::<bool>(&mut conn)
    .map_err(|e| ApiResponse::new_err(
        ErrorCode::Database,
        "Ошибка при работе с БД, попробуйте позже".to_string(),
        Some(e.to_string()),
    ))?;

    if !spec_exists {
        return Err(ApiResponse::new_err(
            ErrorCode::Database,
            "Специализация с указанным ID не найдена".to_string(),
            None,
        ));
    }

    let updated: UserInfoExecutor = diesel::update(user_info_executor::table.find(user.id))
        .set(user_info_executor::specialization.eq(Some(payload.specialization_id)))
        .get_result::<UserInfoExecutor>(&mut conn)
        .map_err(|e| ApiResponse::new_err(
            ErrorCode::Database,
            "Ошибка при работе с БД, попробуйте позже".to_string(),
            Some(e.to_string()),
        ))?;

    Ok(ApiResponse::new_ok(updated))
}

/// Изменение часового пояса
#[utoipa::path(
    patch,
    path = "/api/v1/user/executor/timezone",
    tag = "executor",
    security(("bearer_token" = [])),
    request_body = UpdateTimezoneRequest,
    responses(
        (status = 200, description = "Часовой пояс обновлён", body = ApiResponseExecutor),
        (status = 401, description = "Неавторизован", body = ApiResponseEmpty),
        (status = 500, description = "Ошибка сервера", body = ApiResponseEmpty)
    )
)]
pub async fn update_timezone(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    Json(payload): Json<UpdateTimezoneRequest>,
) -> ApiResult<UserInfoExecutor> {
    let mut conn = state.db_pool.get()
        .map_err(|e| ApiResponse::new_err(
            ErrorCode::Database,
            "Ошибка при подключении к БД, попробуйте позже".to_string(),
            Some(e.to_string()),
        ))?;

    let updated: UserInfoExecutor = diesel::update(user_info_executor::table.find(user.id))
        .set(user_info_executor::timezone.eq(Some(payload.timezone)))
        .get_result(&mut conn)
        .map_err(|e| ApiResponse::new_err(
            ErrorCode::Database,
            "Ошибка при работе с БД, попробуйте позже".to_string(),
            Some(e.to_string()),
        ))?;

    Ok(ApiResponse::new_ok(updated))
}

/// Изменение графика работы
#[utoipa::path(
    patch,
    path = "/api/v1/user/executor/schedule",
    tag = "executor",
    security(("bearer_token" = [])),
    request_body = UpdateWorkScheduleRequest,
    responses(
        (status = 200, description = "График работы обновлён", body = ApiResponseExecutor),
        (status = 401, description = "Неавторизован", body = ApiResponseEmpty),
        (status = 500, description = "Ошибка сервера", body = ApiResponseEmpty)
    )
)]
pub async fn update_schedule(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    Json(payload): Json<UpdateWorkScheduleRequest>,
) -> ApiResult<UserInfoExecutor> {
    let schedule_json: JsonValue = serde_json::to_value(&payload)
        .map_err(|e| ApiResponse::new_err(
            ErrorCode::Database,
            "Ошибка при работе с БД, попробуйте позже".to_string(),
            Some(e.to_string()),
        ))?;

    let mut conn = state.db_pool.get()
        .map_err(|e| ApiResponse::new_err(
            ErrorCode::Database,
            "Ошибка при подключении к БД, попробуйте позже".to_string(),
            Some(e.to_string()),
        ))?;

    let updated: UserInfoExecutor = diesel::update(user_info_executor::table.find(user.id))
        .set(user_info_executor::work_schedule.eq(Some(schedule_json)))
        .get_result(&mut conn)
        .map_err(|e| ApiResponse::new_err(
            ErrorCode::Database,
            "Ошибка при работе с БД, попробуйте позже".to_string(),
            Some(e.to_string()),
        ))?;

    Ok(ApiResponse::new_ok(updated))
}

/// Изменение правил контактов (список кастомных способов связи)
#[utoipa::path(
    patch,
    path = "/api/v1/user/executor/contacts",
    tag = "executor",
    security(("bearer_token" = [])),
    request_body = UpdateContactRulesRequest,
    responses(
        (status = 200, description = "Контакты обновлены", body = ApiResponseExecutor),
        (status = 401, description = "Неавторизован", body = ApiResponseEmpty),
        (status = 500, description = "Ошибка сервера", body = ApiResponseEmpty)
    )
)]
pub async fn update_contacts(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    Json(payload): Json<UpdateContactRulesRequest>,
) -> ApiResult<UserInfoExecutor> {
    let contacts_json: JsonValue = serde_json::to_value(&payload.contacts)
        .map_err(|e| ApiResponse::new_err(
            ErrorCode::Database,
            "Ошибка при работе с БД, попробуйте позже".to_string(),
            Some(e.to_string()),
        ))?;

    let mut conn = state.db_pool.get()
        .map_err(|e| ApiResponse::new_err(
            ErrorCode::Database,
            "Ошибка при подключении к БД, попробуйте позже".to_string(),
            Some(e.to_string()),
        ))?;

    let updated: UserInfoExecutor = diesel::update(user_info_executor::table.find(user.id))
        .set(user_info_executor::contact_rules.eq(Some(contacts_json)))
        .get_result(&mut conn)
        .map_err(|e| ApiResponse::new_err(
            ErrorCode::Database,
            "Ошибка при работе с БД, попробуйте позже".to_string(),
            Some(e.to_string()),
        ))?;

    Ok(ApiResponse::new_ok(updated))
}
