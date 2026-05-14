//! Файл содержит функции работы со справочными таблицами

use axum::extract::State;
use diesel::prelude::*;
use std::sync::Arc;

use crate::{
    state::AppState,
    models::user::{Specialization, ApiResponceSpecializations},
    schema::specializations,
    api_response::{ApiResult, ApiResponse, ErrorCode, ApiResponseEmpty},
};

/// Получение всех справочных данных
#[utoipa::path(
    get,
    path = "/api/v1/reference/specializations",
    tag = "reference",
    responses(
        (status = 200, description = "Справочные данные успешно получены", body = ApiResponceSpecializations),
        (status = 500, description = "Ошибка сервера", body = ApiResponseEmpty)
    )
)]
pub async fn get_specializations(
    State(state): State<Arc<AppState>>,
) -> ApiResult<Vec<Specialization>> {
    let mut conn = state.db_pool.get()
        .map_err(|e| ApiResponse::new_err(
            ErrorCode::Database,
            "Ошибка при подключении к БД".to_string(),
            Some(e.to_string()),
        ))?;

    let specializations: Vec<Specialization> = specializations::table
        .select(Specialization::as_select())
        .order_by(specializations::name_ru)
        .load(&mut conn)
        .map_err(|e| ApiResponse::new_err(
            ErrorCode::Database,
            "Ошибка при получении специализаций".to_string(),
            Some(e.to_string()),
        ))?;

    Ok(ApiResponse::new_ok(specializations))
}
