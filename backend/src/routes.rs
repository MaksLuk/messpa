use axum::{
    Router,
    routing::{get, post, patch, delete, put},
    middleware,
};
use tower_governor::GovernorLayer;
use utoipa_scalar::{Scalar, Servable};
use utoipa::OpenApi;

use std::sync::Arc;

use crate::{
    handlers::{auth, user, admin, executor, reference, team},
    middleware::auth::auth_middleware,
    middleware::admin::admin_middleware,
    middleware::rate_limit::rate_limit_config,
    state::AppState,
    openapi::ApiDoc,
};

pub fn api_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .layer(GovernorLayer::new(rate_limit_config()))
        .nest("/api/v1", compare_routes(state.clone()))
        .route("/api/v1/openapi.json", axum::routing::get(|| async { 
            axum::Json(ApiDoc::openapi()) 
        }))
        .merge(Scalar::with_url("/api/v1/scalar", ApiDoc::openapi()))
}

pub fn compare_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .nest("/auth", auth_routes(state.clone()))
        .nest("/user", user_routes(state.clone()))
        .nest("/admin", admin_routes(state))
        .nest("/reference", reference_routes())
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
        .route("/avatar", patch(user::update_avatar))
        .route("/avatar", delete(user::delete_avatar))
        .route("/banner", patch(user::update_banner))
        .route("/banner", delete(user::delete_banner))
        .route("/email", post(user::initiate_set_email))
        .route("/email/verify", post(user::verify_set_email))
        .route("/telegram", post(user::initiate_set_telegram))
        .route("/telegram/verify", post(user::verify_set_telegram))
        .route("/executor", post(executor::become_executor))
        .route("/executor", delete(executor::stop_beeng_executor))
        .route("/executor/me", get(executor::get_user_executor))
        .route("/executor/specialization", patch(executor::update_specialization))
        .route("/executor/timezone", patch(executor::update_timezone))
        .route("/executor/schedule", patch(executor::update_schedule))
        .route("/executor/contacts", patch(executor::update_contacts))
        .route("/team", post(team::create_team))
        .route("/team/{team_id}", put(team::update_team))
        .route("/team/{team_id}/member/role", post(team::change_team_member_role))
        .route("/team/{team_id}/member/{user_id}", delete(team::remove_team_member))
        .route("/team/{team_id}/leave", delete(team::leave_team))
        .route("/team/{team_id}/invite", post(team::invite_user_to_team))
        .route("/team/invitation/{invitation_id}/accept", post(team::accept_team_invitation))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
}

fn admin_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route("/users", get(admin::user::list_users))
        .route("/user/{user_id}/info", get(admin::user::get_user))
        .route("/user/{user_id}/executor", get(admin::user::get_user_executor))
        .route("/user/{user_id}/logout", post(admin::user::logout_user))
        .route("/user/{user_id}/role", patch(admin::user::change_user_role))
        .route("/specialization", post(admin::reference::specialization::create_specialization))
        .route("/specialization/{id}", patch(admin::reference::specialization::update_specialization))
        .route("/specialization/{id}", delete(admin::reference::specialization::delete_specialization))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            admin_middleware,
        ))
}

fn reference_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/reference/specializations", get(reference::get_specializations))
}
