use axum::{
    extract::{State, Path, Query},
    Json,
};
use diesel::prelude::*;
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

use std::sync::Arc;

use crate::{
    state::AppState,
    models::user::{User, UserRole, UserInfoExecutor},
    schema::{users, refresh_sessions, user_info_executor},
    api_response::{ApiResult, ApiResponse, ErrorCode},
};

/// Получение информации о пользователе
pub async fn get_user(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<Uuid>,
) -> ApiResult<User> {
    let mut conn = state.db_pool.get()
        .map_err(|e| ApiResponse::new_err(
            ErrorCode::Database,
            "Ошибка при подключении к БД, попробуйте позже".to_string(),
            Some(e.to_string()),
        ))?;

    let user: User = users::table
        .filter(users::id.eq(user_id))
        .select(User::as_select())
        .first::<User>(&mut conn)
        .map_err(|e| ApiResponse::new_err(
            ErrorCode::Database,
            "Ошибка при работе с БД, попробуйте позже".to_string(),
            Some(e.to_string()),
        ))?;

    Ok(ApiResponse::new_ok(user))
}

/// Получение информации о пользователе как об исполнителе
pub async fn get_user_executor(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<Uuid>,
) -> ApiResult<UserInfoExecutor> {
    let mut conn = state.db_pool.get()
        .map_err(|e| ApiResponse::new_err(
            ErrorCode::Database,
            "Ошибка при подключении к БД, попробуйте позже".to_string(),
            Some(e.to_string()),
        ))?;

    let user: UserInfoExecutor = user_info_executor::table
        .filter(user_info_executor::user_id.eq(user_id))
        .select(UserInfoExecutor::as_select())
        .first::<UserInfoExecutor>(&mut conn)
        .map_err(|e| ApiResponse::new_err(
            ErrorCode::Database,
            "Ошибка при работе с БД, попробуйте позже".to_string(),
            Some(e.to_string()),
        ))?;

    Ok(ApiResponse::new_ok(user))
}

/// Завершение всех сессий пользователя
pub async fn logout_user(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<Uuid>,
) -> ApiResult<()> {
    let mut conn = state.db_pool.get()
        .map_err(|e| ApiResponse::new_err(
            ErrorCode::Database,
            "Ошибка при подключении к БД, попробуйте позже".to_string(),
            Some(e.to_string()),
        ))?;

    diesel::update(refresh_sessions::table.filter(refresh_sessions::user_id.eq(user_id)))
        .set(refresh_sessions::revoked.eq(true))
        .execute(&mut conn)
        .map_err(|e| ApiResponse::new_err(
            ErrorCode::Database,
            "Ошибка при работе с БД, попробуйте позже".to_string(),
            Some(e.to_string()),
        ))?;

    Ok(ApiResponse::new_ok(()))
}

#[derive(Deserialize)]
pub struct UpdateRolePayload {
    pub new_role: UserRole,
}

/// Изменение роли пользователя
pub async fn change_user_role(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<Uuid>,
    Json(payload): Json<UpdateRolePayload>,
) -> ApiResult<()> {
    let mut conn = state.db_pool.get()
        .map_err(|e| ApiResponse::new_err(
            ErrorCode::Database,
            "Ошибка при подключении к БД, попробуйте позже".to_string(),
            Some(e.to_string()),
        ))?;

    diesel::update(users::table.find(user_id))
        .set(users::role.eq(&payload.new_role))
        .execute(&mut conn)
        .map_err(|e| ApiResponse::new_err(
            ErrorCode::Database,
            "Ошибка при работе с БД, попробуйте позже".to_string(),
            Some(e.to_string()),
        ))?;

    Ok(ApiResponse::new_ok(()))
}

// Получение списка пользователей

#[derive(Debug, Deserialize)]
pub struct ListUsersQuery {
    /// Курсор для следующей страницы (берётся из `next_cursor` предыдущего ответа)
    pub cursor: Option<String>,
    /// Количество записей на странице (по умолчанию 20, максимум 100)
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, Clone)]
pub struct ListUsersResponse {
    pub data: Vec<User>,
    /// Курсор для следующей страницы. `null` — если пользователей больше нет
    pub next_cursor: Option<String>,
}

fn create_cursor(register_at: Option<DateTime<Utc>>, id: Uuid) -> String {
    use base64::{engine::general_purpose, Engine as _};

    let ts = register_at
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_else(|| "null".to_string());

    let raw = format!("{}|{}", ts, id);
    general_purpose::URL_SAFE_NO_PAD.encode(raw.as_bytes())
}

fn parse_cursor(cursor: &str) -> Result<(Option<DateTime<Utc>>, Uuid), String> {
    use base64::{engine::general_purpose, Engine as _};

    let decoded = general_purpose::URL_SAFE_NO_PAD
        .decode(cursor)
        .map_err(|e| format!("Invalid cursor encoding: {e}"))?;

    let s = String::from_utf8(decoded).map_err(|e| e.to_string())?;
    let parts: Vec<&str> = s.splitn(2, '|').collect();

    if parts.len() != 2 {
        return Err("Invalid cursor format".to_string());
    }

    let reg_str = parts[0];
    let id_str = parts[1];

    let register_at = if reg_str == "null" || reg_str.is_empty() {
        None
    } else {
        Some(
            DateTime::parse_from_rfc3339(reg_str)
                .map_err(|e| e.to_string())?
                .with_timezone(&Utc),
        )
    };

    let id = Uuid::parse_str(id_str).map_err(|e| e.to_string())?;
    Ok((register_at, id))
}

pub async fn list_users(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ListUsersQuery>,
) -> ApiResult<ListUsersResponse> {
    let limit = query.limit.unwrap_or(20).clamp(1, 100) as i64;

    let cursor = if let Some(c) = &query.cursor {
        match parse_cursor(c) {
            Ok(c) => Some(c),
            Err(e) => {
                return Err(ApiResponse::new_err(
                    ErrorCode::Validation,
                    "Неверный формат курсора".to_string(),
                    Some(e),
                ));
            }
        }
    } else {
        None
    };

    let mut conn = state.db_pool.get().map_err(|e| {
        ApiResponse::new_err(
            ErrorCode::Database,
            "Ошибка при подключении к БД, попробуйте позже".to_string(),
            Some(e.to_string()),
        )
    })?;

    let mut query = users::table
        .select(User::as_select())
        .order_by((
            users::register_at.desc().nulls_last(),
            users::id.desc(),
        ))
        .limit(limit)
        .into_boxed();

    if let Some((last_register_at, last_id)) = cursor {
        if let Some(last_reg) = last_register_at {
            query = query.filter(
                users::register_at
                    .lt(last_reg)
                    .or(users::register_at.eq(last_reg).and(users::id.lt(last_id))),
            );
        } else {
            query = query.filter(
                users::register_at.is_null().and(users::id.lt(last_id)),
            );
        }
    }

    let users: Vec<User> = query.load::<User>(&mut conn).map_err(|e| {
        ApiResponse::new_err(
            ErrorCode::Database,
            "Ошибка при работе с БД, попробуйте позже".to_string(),
            Some(e.to_string()),
        )
    })?;

    let next_cursor = if users.len() == limit as usize {
        users.last().map(|u| create_cursor(u.register_at, u.id))
    } else {
        None
    };

    Ok(ApiResponse::new_ok(ListUsersResponse {
        data: users,
        next_cursor,
    }))
}
