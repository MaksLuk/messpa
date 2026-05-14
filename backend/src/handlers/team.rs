use axum::{
    extract::{State, Extension, Path},
    Json,
};
use chrono::Utc;
use diesel::prelude::*;
use serde::Deserialize;
use std::sync::Arc;

use crate::{
    state::AppState,
    models::user::{
        User, Team, TeamRole,
        NewTeam, TeamMember, NewTeamMember, UpdateTeam,
        ApiResponseTeam, ApiResponseTeamMember,
        TeamInvitation, ApiResponseTeamInvitation, NewTeamInvitation,
        InvitationStatus,
    },
    schema::{teams, team_members, specializations, team_invitations},
    api_response::{ApiResult, ApiResponse, ErrorCode, ApiResponseEmpty},
};

/// Разрешённые ключи для public_contacts
const ALLOWED_PUBLIC_CONTACT_KEYS: &[&str] = &[
    "telegram", "email", "vk", "whatsapp", "phone",
    "discord", "twitter", "instagram", "facebook",
    "youtube", "website", "linkedin", "github",
];

fn validate_public_contacts(contacts: &serde_json::Value) -> Result<(), ApiResponse<()>> {
    if let serde_json::Value::Object(map) = contacts {
        for (key, value) in map {
            if !ALLOWED_PUBLIC_CONTACT_KEYS.contains(&key.as_str()) {
                return Err(ApiResponse::new_err(
                    ErrorCode::Validation,
                    format!("Недопустимый ключ контакта: {}", key),
                    None,
                ));
            }
            if !value.is_string() {
                return Err(ApiResponse::new_err(
                    ErrorCode::Validation,
                    format!("Значение ключа '{}' должно быть строкой", key),
                    None,
                ));
            }
        }
        Ok(())
    } else {
        Err(ApiResponse::new_err(
            ErrorCode::Validation,
            "Поле public_contacts должно быть объектом".to_string(),
            None,
        ))
    }
}

fn validate_specializations(
    conn: &mut PgConnection,
    specs: &[Option<i32>],
) -> Result<(), ApiResponse<()>> {
    let ids: Vec<i32> = specs.iter().filter_map(|&x| x).collect();
    if ids.is_empty() {
        return Ok(());
    }

    let count: i64 = specializations::table
        .filter(specializations::id.eq_any(&ids))
        .count()
        .get_result(conn)
        .map_err(|e| ApiResponse::new_err(
            ErrorCode::Database,
            "Ошибка проверки специализаций".to_string(),
            Some(e.to_string()),
        ))?;

    if count as usize != ids.len() {
        Err(ApiResponse::new_err(
            ErrorCode::Validation,
            "Одна или несколько специализаций не существуют".to_string(),
            None,
        ))
    } else {
        Ok(())
    }
}

fn get_member_role(
    conn: &mut PgConnection,
    team_id: uuid::Uuid,
    user_id: uuid::Uuid,
) -> Result<TeamRole, diesel::result::Error> {
    team_members::table
        .filter(team_members::team_id.eq(team_id))
        .filter(team_members::user_id.eq(user_id))
        .select(team_members::role)
        .first(conn)
}

fn get_member_role_or_err(
    conn: &mut PgConnection,
    team_id: uuid::Uuid,
    user_id: uuid::Uuid,
) -> Result<TeamRole, ApiResponse<()>> {
    match get_member_role(conn, team_id, user_id) {
        Ok(role) => Ok(role),
        Err(diesel::result::Error::NotFound) => Err(ApiResponse::new_err(
            ErrorCode::Forbidden,
            "Вы не являетесь участником этой команды".to_string(),
            None,
        )),
        Err(e) => Err(ApiResponse::new_err(
            ErrorCode::Database,
            "Ошибка базы данных".to_string(),
            Some(e.to_string()),
        )),
    }
}

fn can_set_role(actor_role: TeamRole, new_role: TeamRole) -> bool {
    match actor_role {
        TeamRole::Owner => true,
        TeamRole::Admin => matches!(new_role, TeamRole::Manager | TeamRole::Executor),
        TeamRole::Manager => matches!(new_role, TeamRole::Executor),
        TeamRole::Executor => false,
    }
}

fn can_remove_member(actor_role: TeamRole, target_role: TeamRole) -> bool {
    match actor_role {
        TeamRole::Owner => true,
        TeamRole::Admin => matches!(target_role, TeamRole::Manager | TeamRole::Executor),
        TeamRole::Manager => matches!(target_role, TeamRole::Executor),
        TeamRole::Executor => false,
    }
}

#[axum::debug_handler]
#[utoipa::path(
    post,
    path = "/api/v1/team",
    tag = "team",
    security(("bearer_token" = [])),
    request_body = NewTeam,
    responses(
        (status = 200, description = "Команда успешно создана", body = ApiResponseTeam),
        (status = 401, description = "Неавторизован", body = ApiResponseEmpty),
        (status = 422, description = "Ошибка валидации", body = ApiResponseEmpty),
        (status = 500, description = "Ошибка сервера", body = ApiResponseEmpty)
    )
)]
pub async fn create_team(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    Json(new_team_payload): Json<NewTeam>,
) -> ApiResult<Team> {
    let mut conn = state.db_pool.get().map_err(|e| ApiResponse::new_err(
        ErrorCode::Database,
        "Ошибка при подключении к БД, попробуйте позже".to_string(),
        Some(e.to_string()),
    ))?;

    // === ВАЛИДАЦИЯ ===
    if let Some(specs) = &new_team_payload.specializations {
        validate_specializations(&mut conn, specs)?;
    }
    if let Some(contacts) = &new_team_payload.public_contacts {
        validate_public_contacts(contacts)?;
    }

    // === Создаём команду ===
    let team: Team = diesel::insert_into(teams::table)
        .values(&new_team_payload)
        .get_result(&mut conn)
        .map_err(|e| ApiResponse::new_err(
            ErrorCode::Database,
            "Ошибка при создании команды".to_string(),
            Some(e.to_string()),
        ))?;

    // === Добавляем создателя как Owner ===
    let now = Utc::now();
    let new_member = NewTeamMember {
        team_id: team.id,
        user_id: user.id,
        role: TeamRole::Owner,
        joined_at: Some(now),
    };

    diesel::insert_into(team_members::table)
        .values(&new_member)
        .execute(&mut conn)
        .map_err(|e| ApiResponse::new_err(
            ErrorCode::Database,
            "Ошибка при добавлении владельца команды".to_string(),
            Some(e.to_string()),
        ))?;

    Ok(ApiResponse::new_ok(team))
}

#[utoipa::path(
    put,
    path = "/api/v1/team/{team_id}",
    tag = "team",
    security(("bearer_token" = [])),
    params(("team_id" = uuid::Uuid, Path, description = "ID команды")),
    request_body = UpdateTeam,
    responses(
        (status = 200, description = "Команда обновлена", body = ApiResponseTeam),
        (status = 401, description = "Неавторизован", body = ApiResponseEmpty),
        (status = 403, description = "Нет прав", body = ApiResponseEmpty),
        (status = 500, description = "Ошибка сервера", body = ApiResponseEmpty)
    )
)]
pub async fn update_team(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    Path(team_id): Path<uuid::Uuid>,
    Json(payload): Json<UpdateTeam>,
) -> ApiResult<Team> {
    let mut conn = state.db_pool.get().map_err(|e| ApiResponse::new_err(
        ErrorCode::Database,
        "Ошибка подключения к БД".to_string(),
        Some(e.to_string()),
    ))?;

    let actor_role = get_member_role_or_err(&mut conn, team_id, user.id)?;

    if !matches!(actor_role, TeamRole::Owner | TeamRole::Admin) {
        return Err(ApiResponse::new_err(
            ErrorCode::Forbidden,
            "Редактировать команду могут только владелец и администратор".to_string(),
            None,
        ));
    }

    // Валидация
    if let Some(specs) = &payload.specializations {
        validate_specializations(&mut conn, specs)?;
    }
    if let Some(contacts) = &payload.public_contacts {
        validate_public_contacts(contacts)?;
    }

    // Проверяем, есть ли хоть одно поле для обновления
    let has_changes = payload.name.is_some()
        || payload.description.is_some()
        || payload.banner_url.is_some()
        || payload.logo_url.is_some()
        || payload.specializations.is_some()
        || payload.public_contacts.is_some();

    if !has_changes {
        let team: Team = teams::table
            .find(team_id)
            .first(&mut conn)
            .map_err(|_| ApiResponse::new_err(
                ErrorCode::Validation,
                "Нет данных для обновления".to_string(),
                None,
            ))?;
        return Ok(ApiResponse::new_ok(team));
    }

    let update_data = UpdateTeam {
        name: payload.name,
        description: payload.description,
        banner_url: payload.banner_url,
        logo_url: payload.logo_url,
        specializations: payload.specializations,
        public_contacts: payload.public_contacts,
        updated_at: Some(Utc::now()),
    };

    let updated: Team = diesel::update(teams::table.filter(teams::id.eq(team_id)))
        .set(&update_data)
        .get_result(&mut conn)
        .map_err(|e| ApiResponse::new_err(
            ErrorCode::Database,
            "Ошибка обновления команды".to_string(),
            Some(e.to_string()),
        ))?;

    Ok(ApiResponse::new_ok(updated))
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct ChangeRoleRequest {
    pub user_id: uuid::Uuid,
    pub role: TeamRole,
}

#[utoipa::path(
    post,
    path = "/api/v1/team/{team_id}/member/role",
    tag = "team",
    security(("bearer_token" = [])),
    params(("team_id" = uuid::Uuid, Path)),
    request_body = ChangeRoleRequest,
    responses(
        (status = 200, description = "Пользователь исключен из команды", body = ApiResponseTeamMember),
        (status = 401, description = "Неавторизован", body = ApiResponseEmpty),
        (status = 403, description = "Нет прав", body = ApiResponseEmpty),
        (status = 500, description = "Ошибка сервера", body = ApiResponseEmpty)
    )
)]
pub async fn change_team_member_role(
    State(state): State<Arc<AppState>>,
    Extension(actor): Extension<User>,
    Path(team_id): Path<uuid::Uuid>,
    Json(req): Json<ChangeRoleRequest>,
) -> ApiResult<TeamMember> {
    let mut conn = state.db_pool.get().map_err(|e| ApiResponse::new_err(
        ErrorCode::Database,
        "Ошибка подключения к БД".to_string(),
        Some(e.to_string()),
    ))?;

    if req.user_id == actor.id {
        return Err(ApiResponse::new_err(
            ErrorCode::Validation,
            "Нельзя менять роль самому себе".to_string(),
            None,
        ));
    }

    let actor_role = get_member_role_or_err(&mut conn, team_id, actor.id)?;
    let _target_current_role = get_member_role_or_err(&mut conn, team_id, req.user_id)?;

    if req.role == TeamRole::Owner {
        if actor_role != TeamRole::Owner {
            return Err(ApiResponse::new_err(
                ErrorCode::Forbidden,
                "Назначать владельца может только текущий владелец".to_string(),
                None,
            ));
        }
        // Демотируем себя до Admin
        diesel::update(team_members::table
            .filter(team_members::team_id.eq(team_id))
            .filter(team_members::user_id.eq(actor.id))
        )
        .set(team_members::role.eq(TeamRole::Admin))
        .execute(&mut conn)
        .map_err(|e| ApiResponse::new_err(ErrorCode::Database, "Ошибка демотирования".to_string(), Some(e.to_string())))?;
    } else if !can_set_role(actor_role, req.role) {
        return Err(ApiResponse::new_err(
            ErrorCode::Forbidden,
            "Недостаточно прав для назначения этой роли".to_string(),
            None,
        ));
    }

    // Обновляем роль целевого пользователя
    let updated: TeamMember = diesel::update(team_members::table
        .filter(team_members::team_id.eq(team_id))
        .filter(team_members::user_id.eq(req.user_id))
    )
    .set(team_members::role.eq(req.role))
    .get_result(&mut conn)
    .map_err(|e| ApiResponse::new_err(
        ErrorCode::Database,
        "Ошибка изменения роли".to_string(),
        Some(e.to_string()),
    ))?;

    Ok(ApiResponse::new_ok(updated))
}

#[utoipa::path(
    delete,
    path = "/api/v1/team/{team_id}/member/{user_id}",
    tag = "team",
    security(("bearer_token" = [])),
    params(
        ("team_id" = uuid::Uuid, Path),
        ("user_id" = uuid::Uuid, Path)
    ),
    responses(
        (status = 200, description = "Пользователь исключен из команды", body = ApiResponseTeamMember),
        (status = 403, description = "Нет прав", body = ApiResponseEmpty),
        (status = 500, description = "Ошибка сервера", body = ApiResponseEmpty)
    )
)]
pub async fn remove_team_member(
    State(state): State<Arc<AppState>>,
    Extension(actor): Extension<User>,
    Path((team_id, target_user_id)): Path<(uuid::Uuid, uuid::Uuid)>,
) -> ApiResult<()> {
    let mut conn = state.db_pool.get().map_err(|e| ApiResponse::new_err(
        ErrorCode::Database,
        "Ошибка подключения к БД".to_string(),
        Some(e.to_string()),
    ))?;

    if actor.id == target_user_id {
        return Err(ApiResponse::new_err(
            ErrorCode::Validation,
            "Нельзя исключить самого себя из команды".to_string(),
            None,
        ));
    }

    let actor_role = get_member_role_or_err(&mut conn, team_id, actor.id)?;
    let target_role = get_member_role_or_err(&mut conn, team_id, target_user_id)?;

    if !can_remove_member(actor_role, target_role) {
        return Err(ApiResponse::new_err(
            ErrorCode::Forbidden,
            "Недостаточно прав для исключения этого участника".to_string(),
            None,
        ));
    }

    diesel::delete(team_members::table
        .filter(team_members::team_id.eq(team_id))
        .filter(team_members::user_id.eq(target_user_id))
    )
    .execute(&mut conn)
    .map_err(|e| ApiResponse::new_err(
        ErrorCode::Database,
        "Ошибка исключения участника".to_string(),
        Some(e.to_string()),
    ))?;

    Ok(ApiResponse::new_ok(()))
}

#[utoipa::path(
    delete,
    path = "/api/v1/team/{team_id}/leave",
    tag = "team",
    security(("bearer_token" = [])),
    params(
        ("team_id" = uuid::Uuid, Path, description = "ID команды")
    ),
    responses(
        (status = 200, description = "Вы успешно покинули команду", body = ApiResponseEmpty),
        (status = 401, description = "Неавторизован", body = ApiResponseEmpty),
        (status = 403, description = "Владелец не может покинуть команду", body = ApiResponseEmpty),
        (status = 404, description = "Вы не являетесь участником команды", body = ApiResponseEmpty),
        (status = 500, description = "Ошибка сервера", body = ApiResponseEmpty)
    )
)]
pub async fn leave_team(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    Path(team_id): Path<uuid::Uuid>,
) -> ApiResult<()> {
    let mut conn = state.db_pool.get().map_err(|e| ApiResponse::new_err(
        ErrorCode::Database,
        "Ошибка подключения к БД".to_string(),
        Some(e.to_string()),
    ))?;

    // Получаем роль текущего пользователя в команде
    let actor_role = match get_member_role(&mut conn, team_id, user.id) {
        Ok(role) => role,
        Err(diesel::result::Error::NotFound) => {
            return Err(ApiResponse::new_err(
                ErrorCode::NotFound,
                "Вы не являетесь участником этой команды".to_string(),
                None,
            ));
        }
        Err(e) => {
            return Err(ApiResponse::new_err(
                ErrorCode::Database,
                "Ошибка базы данных".to_string(),
                Some(e.to_string()),
            ));
        }
    };

    // Владелец не может покинуть команду
    if actor_role == TeamRole::Owner {
        return Err(ApiResponse::new_err(
            ErrorCode::Forbidden,
            "Владелец не может покинуть команду. Сначала передайте владение другому участнику.".to_string(),
            None,
        ));
    }

    // Удаляем пользователя из команды
    let deleted_count = diesel::delete(team_members::table
        .filter(team_members::team_id.eq(team_id))
        .filter(team_members::user_id.eq(user.id))
    )
    .execute(&mut conn)
    .map_err(|e| ApiResponse::new_err(
        ErrorCode::Database,
        "Ошибка при выходе из команды".to_string(),
        Some(e.to_string()),
    ))?;

    if deleted_count == 0 {
        return Err(ApiResponse::new_err(
            ErrorCode::NotFound,
            "Вы не являетесь участником этой команды".to_string(),
            None,
        ));
    }

    Ok(ApiResponse::new_ok(()))
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct InviteUserRequest {
    pub user_id: uuid::Uuid,           // кого приглашаем
    pub role: TeamRole,          // какую роль дать
}

#[utoipa::path(
    post,
    path = "/api/v1/team/{team_id}/invite",
    tag = "team",
    security(("bearer_token" = [])),
    params(("team_id" = uuid::Uuid, Path)),
    request_body = InviteUserRequest,
    responses(
        (status = 200, description = "Вы успешно покинули команду", body = ApiResponseTeamInvitation),
        (status = 403, description = "Нет прав", body = ApiResponseEmpty),
        (status = 500, description = "Ошибка сервера", body = ApiResponseEmpty)
    )
)]
pub async fn invite_user_to_team(
    State(state): State<Arc<AppState>>,
    Extension(inviter): Extension<User>,
    Path(team_id): Path<uuid::Uuid>,
    Json(payload): Json<InviteUserRequest>,
) -> ApiResult<TeamInvitation> {
    let mut conn = state.db_pool.get().map_err(|e| ApiResponse::new_err(
        ErrorCode::Database,
        "Ошибка подключения к БД".to_string(),
        Some(e.to_string()),
    ))?;

    // Проверка прав (только Owner и Admin)
    let inviter_role = get_member_role_or_err(&mut conn, team_id, inviter.id)?;
    if !matches!(inviter_role, TeamRole::Owner | TeamRole::Admin) {
        return Err(ApiResponse::new_err(
            ErrorCode::Forbidden,
            "Приглашать пользователей могут только владелец и администратор".to_string(),
            None,
        ));
    }

    // Проверяем, не состоит ли уже пользователь в команде
    let already_member = team_members::table
        .filter(team_members::team_id.eq(team_id))
        .filter(team_members::user_id.eq(payload.user_id))
        .count()
        .get_result::<i64>(&mut conn)
        .unwrap_or(0) > 0;

    if already_member {
        return Err(ApiResponse::new_err(
            ErrorCode::Conflict,
            "Пользователь уже состоит в команде".to_string(),
            None,
        ));
    }

    // Проверяем, нет ли уже активного приглашения
    let existing_invite: Option<TeamInvitation> = team_invitations::table
        .filter(team_invitations::team_id.eq(team_id))
        .filter(team_invitations::invitee_id.eq(payload.user_id))
        .filter(team_invitations::status.eq(InvitationStatus::Pending))
        .first(&mut conn)
        .optional()
        .map_err(|e| ApiResponse::new_err(
            ErrorCode::Conflict,
            "Пользователь уже получил приглашение".to_string(),
            Some(e.to_string()),
        ))?;

    if existing_invite.is_some() {
        return Err(ApiResponse::new_err(
            ErrorCode::Conflict,
            "Приглашение этому пользователю уже отправлено".to_string(),
            None,
        ));
    }

    // Создаём приглашение (срок действия 7 дней)
    let expires_at = Utc::now() + chrono::Duration::days(7);

    let new_invite = NewTeamInvitation {
        team_id,
        inviter_id: inviter.id,
        invitee_id: payload.user_id,
        role: payload.role,
        status: InvitationStatus::Pending,
        expires_at: Some(expires_at),
    };

    let invitation: TeamInvitation = diesel::insert_into(team_invitations::table)
        .values(&new_invite)
        .get_result(&mut conn)
        .map_err(|e| ApiResponse::new_err(
            ErrorCode::Database,
            "Ошибка создания приглашения".to_string(),
            Some(e.to_string()),
        ))?;

    Ok(ApiResponse::new_ok(invitation))
}

#[utoipa::path(
    post,
    path = "/api/v1/team/invitation/{invitation_id}/accept",
    tag = "team",
    security(("bearer_token" = [])),
    params(("invitation_id" = uuid::Uuid, Path)),
    responses(
        (status = 200, description = "Вы успешно покинули команду", body = ApiResponseTeamMember),
        (status = 500, description = "Ошибка сервера", body = ApiResponseEmpty)
    )
)]
pub async fn accept_team_invitation(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    Path(invitation_id): Path<uuid::Uuid>,
) -> ApiResult<TeamMember> {
    let mut conn = state.db_pool.get().map_err(|e| ApiResponse::new_err(
        ErrorCode::Database,
        "Ошибка подключения к БД".to_string(),
        Some(e.to_string()),
    ))?;

    // Находим приглашение
    let invitation: TeamInvitation = team_invitations::table
        .find(invitation_id)
        .first(&mut conn)
        .map_err(|_| ApiResponse::new_err(
            ErrorCode::NotFound,
            "Приглашение не найдено".to_string(),
            None,
        ))?;

    // Проверяем, что приглашение адресовано именно этому пользователю
    if invitation.invitee_id != user.id {
        return Err(ApiResponse::new_err(
            ErrorCode::Forbidden,
            "Это приглашение адресовано другому пользователю".to_string(),
            None,
        ));
    }

    if invitation.status != InvitationStatus::Pending {
        return Err(ApiResponse::new_err(
            ErrorCode::Conflict,
            "Приглашение уже принято или отклонено".to_string(),
            None,
        ));
    }

    // Проверяем срок действия
    if let Some(expires_at) = invitation.expires_at {
        if Utc::now() > expires_at {
            // Помечаем как expired
            diesel::update(team_invitations::table.find(invitation_id))
                .set(team_invitations::status.eq(InvitationStatus::Expired))
                .execute(&mut conn)
                .map_err(|e| ApiResponse::new_err(
                    ErrorCode::Database,
                    "Ошибка при изменении данных в БД".to_string(),
                    Some(e.to_string()),
                ))?;
            return Err(ApiResponse::new_err(
                ErrorCode::Conflict,
                "Срок действия приглашения истёк".to_string(),
                None,
            ));
        }
    }

    // Добавляем пользователя в команду
    let new_member = NewTeamMember {
        team_id: invitation.team_id,
        user_id: user.id,
        role: invitation.role,
        joined_at: Some(Utc::now()),
    };

    let member: TeamMember = diesel::insert_into(team_members::table)
        .values(&new_member)
        .get_result(&mut conn)
        .map_err(|e| ApiResponse::new_err(
            ErrorCode::Database,
            "Ошибка при изменении данных в БД".to_string(),
            Some(e.to_string()),
        ))?;

    // Обновляем статус приглашения
    diesel::update(team_invitations::table.find(invitation_id))
        .set((
            team_invitations::status.eq(InvitationStatus::Accepted),
            team_invitations::updated_at.eq(Utc::now()),
        ))
        .execute(&mut conn)
        .map_err(|e| ApiResponse::new_err(
            ErrorCode::Database,
            "Ошибка при изменении данных в БД".to_string(),
            Some(e.to_string()),
        ))?;

    Ok(ApiResponse::new_ok(member))
}

