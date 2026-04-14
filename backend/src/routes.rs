use axum::{
    Router,
    routing::{get, post, patch},
    middleware,
};
use tower_governor::GovernorLayer;

use std::sync::Arc;

use crate::{
    handlers::{auth, user},
    middleware::auth::auth_middleware,
    middleware::rate_limit::rate_limit_config,
    state::AppState,
};

pub fn api_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .layer(GovernorLayer::new(rate_limit_config()))
        .nest("/api/auth", auth_routes(state.clone()))
        .nest("/api/user", user_routes(state))
}

fn auth_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    let public_routes = Router::new()
        .route("/telegram/send-code", post(auth::send_telegram_code))
        .route("/telegram/verify", post(auth::verify_telegram_code))
        .route("/email/send-code", post(auth::send_email_code))
        .route("/email/verify", post(auth::verify_email_code))
        .route("/refresh", post(auth::refresh_token))
        .route("/logout", post(auth::logout));

    let protected_routes = Router::new()
        .route("/logout-all", post(auth::logout_all))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));

    public_routes.merge(protected_routes)
}

fn user_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route("/me", get(user::get_current_user))
        .route("/display-name", patch(user::update_display_name))
        .route("/language", patch(user::update_language))
        .route("/currency", patch(user::update_currency))
        .route("/email", post(user::initiate_set_email))
        .route("/email/verify", post(user::verify_set_email))
        .route("/telegram", post(user::initiate_set_telegram))
        .route("/telegram/verify", post(user::verify_set_telegram))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
}
