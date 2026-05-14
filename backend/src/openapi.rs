use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Messpa API",
        description = "Messpa API Docs",
        version = "0.1.0"
    ),
    tags(
        (name = "auth", description = "Аутентификация и авторизация"),
        (name = "user", description = "Управление профилем пользователя")
    ),
    paths(
        // Auth
        crate::handlers::auth::send_telegram_code,
        crate::handlers::auth::verify_telegram_code,
        crate::handlers::auth::send_email_code,
        crate::handlers::auth::verify_email_code,
        crate::handlers::auth::refresh_token,
        crate::handlers::auth::logout,
        crate::handlers::auth::logout_all,

        // User
        crate::handlers::user::get_current_user,
        crate::handlers::user::update_display_name,
        crate::handlers::user::update_language,
        crate::handlers::user::update_currency,
        crate::handlers::user::update_avatar,
        crate::handlers::user::delete_avatar,
        crate::handlers::user::update_banner,
        crate::handlers::user::delete_banner,
        crate::handlers::user::initiate_set_email,
        crate::handlers::user::verify_set_email,
        crate::handlers::user::initiate_set_telegram,
        crate::handlers::user::verify_set_telegram,

        // Executor
        crate::handlers::executor::become_executor,
        crate::handlers::executor::stop_beeng_executor,
        crate::handlers::executor::update_specialization,
        crate::handlers::executor::update_timezone,
        crate::handlers::executor::update_schedule,
        crate::handlers::executor::update_contacts,
        crate::handlers::executor::get_user_executor,

        // Справочные таблицы
        crate::handlers::reference::get_specializations,

        // Команды
        crate::handlers::team::create_team,
        crate::handlers::team::update_team,
        crate::handlers::team::change_team_member_role,
        crate::handlers::team::remove_team_member,
        crate::handlers::team::leave_team,
        crate::handlers::team::invite_user_to_team,
        crate::handlers::team::accept_team_invitation,
    ),
    components(
        schemas(
            crate::api_response::ApiResponse<crate::handlers::auth::AuthResponse>,
            crate::api_response::ApiResponse<crate::handlers::auth::RefreshResponse>,
            crate::api_response::ApiResponse<crate::handlers::auth::SendCodeResponse>,
            crate::api_response::ApiResponse<crate::handlers::auth::EmailResponse>,
            crate::api_response::ApiResponse<crate::handlers::user::TelegramResponse>,

            crate::api_response::ApiError,
            crate::api_response::ApiMeta,
            crate::api_response::ErrorCode,
            crate::api_response::ApiResponseEmpty,

            crate::handlers::auth::VerifyPayload,
            crate::handlers::auth::SendEmailPayload,
            crate::handlers::user::UpdateDisplayNamePayload,
            crate::handlers::user::UpdateLanguagePayload,
            crate::handlers::user::UpdateCurrencyPayload,
            crate::handlers::user::InitiateEmailPayload,

            crate::models::user::User,
            crate::models::user::ApiResponseUser,
            crate::models::user::UserInfoExecutor,
            crate::models::user::ApiResponseExecutor,
            crate::models::user::Language,
            crate::models::user::Currency,

            crate::handlers::auth::SendCodeResponse,
            crate::handlers::auth::AuthResponse,
            crate::handlers::auth::ApiAuthResponse,
            crate::handlers::auth::RefreshResponse,
            crate::handlers::auth::ApiRefreshResponse,
            crate::handlers::auth::EmailResponse,
            crate::handlers::user::TelegramResponse,
        )
    ),
    modifiers(&SecurityAddon)
)]
pub struct ApiDoc;

// Для Bearer JWT
struct SecurityAddon;

impl utoipa::Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_token",
                utoipa::openapi::security::SecurityScheme::Http(
                    utoipa::openapi::security::HttpBuilder::new()
                        .scheme(utoipa::openapi::security::HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .build(),
                ),
            );
        }
    }
}

