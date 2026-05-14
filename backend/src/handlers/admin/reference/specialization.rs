use axum::{
    extract::{State, Path},
    Json,
};
use diesel::prelude::*;
use serde::Deserialize;
use std::sync::Arc;

use crate::{
    state::AppState,
    models::user::{Specialization, NewSpecialization},
    schema::specializations,
    api_response::{ApiResult, ApiResponse, ErrorCode},
};

#[derive(Deserialize)]
pub struct CreateSpecializationRequest {
    pub name_ru: String,
    pub name_en: String,
}

#[derive(Deserialize)]
pub struct UpdateSpecializationRequest {
    pub name_ru: Option<String>,
    pub name_en: Option<String>,
}

/// Создание новой специализации
pub async fn create_specialization(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateSpecializationRequest>,
) -> ApiResult<Specialization> {
    let mut conn = state.db_pool.get()
        .map_err(|e| ApiResponse::new_err(ErrorCode::Database, "Ошибка подключения к БД".into(), Some(e.to_string())))?;

    let new_spec = NewSpecialization {
        name_ru: payload.name_ru,
        name_en: payload.name_en,
    };

    let result: Specialization = diesel::insert_into(specializations::table)
        .values(&new_spec)
        .get_result(&mut conn)
        .map_err(|e| ApiResponse::new_err(
            ErrorCode::Database,
            "Ошибка при создании специализации".to_string(),
            Some(e.to_string()),
        ))?;

    Ok(ApiResponse::new_ok(result))
}

/// Обновление специализации
pub async fn update_specialization(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateSpecializationRequest>,
) -> ApiResult<Specialization> {
    let mut conn = state.db_pool.get()
        .map_err(|e| ApiResponse::new_err(ErrorCode::Database, "Ошибка подключения".into(), Some(e.to_string())))?;

    let updated: Specialization = diesel::update(specializations::table.find(id))
        .set((
            payload.name_ru.map(|v| specializations::name_ru.eq(v)),
            payload.name_en.map(|v| specializations::name_en.eq(v)),
        ))
        .get_result(&mut conn)
        .map_err(|e| match e {
            diesel::result::Error::NotFound => ApiResponse::new_err(
                ErrorCode::NotFound,
                "Специализация не найдена".to_string(),
                None,
            ),
            _ => ApiResponse::new_err(
                ErrorCode::Database,
                "Ошибка при обновлении".to_string(),
                Some(e.to_string()),
            ),
        })?;

    Ok(ApiResponse::new_ok(updated))
}

/// Удаление специализации
pub async fn delete_specialization(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
) -> ApiResult<()> {
    let mut conn = state.db_pool.get()
        .map_err(|e| ApiResponse::new_err(ErrorCode::Database, "Ошибка подключения".into(), Some(e.to_string())))?;

    let deleted = diesel::delete(specializations::table.find(id))
        .execute(&mut conn)
        .map_err(|e| ApiResponse::new_err(
            ErrorCode::Database,
            "Ошибка при удалении".to_string(),
            Some(e.to_string()),
        ))?;

    if deleted == 0 {
        return Err(ApiResponse::new_err(
            ErrorCode::NotFound,
            "Специализация не найдена".to_string(),
            None,
        ));
    }

    Ok(ApiResponse::new_ok(()))
}
