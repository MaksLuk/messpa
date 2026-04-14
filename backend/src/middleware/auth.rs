use axum::{
    extract::{Request, State},
    middleware::Next,
    response::IntoResponse,
};
use diesel::prelude::*;
use axum_extra::headers::{authorization::Bearer, Authorization};
use axum_extra::TypedHeader;
use std::sync::Arc;

use crate::{
    state::AppState,
    models::user::User,
    utils::jwt,
    error::AppError,
    schema::users,
};

pub async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    mut req: Request,
    next: Next,
) -> Result<impl IntoResponse, AppError> {
    let token = auth.token();

    // Проверка и декодирование токена
    let data: jwt::Claims = jwt::validate_access_token(token, &state.config.jwt_secret)
        .map_err(|e| AppError::Auth(format!("Недействительный токен: {}", e)))?;
    let user_id = data.sub;

    // Получение пользователя из БД
    let mut conn = state.db_pool.get()
        .map_err(|e| AppError::Pool(e.into()))?;

    let user: User = users::table
        .filter(users::id.eq(user_id))
        .select(User::as_select())
        .first::<User>(&mut conn)
        .map_err(|_| AppError::Auth("Пользователь не найден".into()))?;

    // Добавление пользователя в extensions
    req.extensions_mut().insert(user);

    Ok(next.run(req).await)
}
