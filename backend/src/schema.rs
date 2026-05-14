// @generated automatically by Diesel CLI.

pub mod sql_types {
    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "currencies"))]
    pub struct Currencies;

    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "languages"))]
    pub struct Languages;

    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "roles"))]
    pub struct Roles;

    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "team_roles"))]
    pub struct TeamRoles;

    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "invitation_status"))]
    pub struct InvitationStatus;
}

diesel::table! {
    specializations (id) {
        id -> Int4,
        #[max_length = 25]
        name_ru -> Varchar,
        #[max_length = 25]
        name_en -> Varchar,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::TeamRoles;

    team_members (team_id, user_id) {
        team_id -> Uuid,
        user_id -> Uuid,
        role -> TeamRoles,
        joined_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    teams (id) {
        id -> Uuid,
        #[max_length = 100]
        name -> Varchar,
        description -> Nullable<Text>,
        banner_url -> Nullable<Text>,
        logo_url -> Nullable<Text>,
        specializations -> Nullable<Array<Nullable<Int4>>>,
        public_contacts -> Nullable<Jsonb>,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    user_info_executor (user_id) {
        user_id -> Uuid,
        specialization -> Nullable<Int4>,
        rating -> Nullable<Numeric>,
        review_count -> Nullable<Int4>,
        completed_orders -> Nullable<Int4>,
        #[max_length = 50]
        timezone -> Nullable<Varchar>,
        work_schedule -> Nullable<Jsonb>,
        contact_rules -> Nullable<Jsonb>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::Roles;
    use super::sql_types::Languages;
    use super::sql_types::Currencies;

    users (id) {
        id -> Uuid,
        #[max_length = 255]
        email -> Nullable<Varchar>,
        role -> Roles,
        telegram_id -> Nullable<Int8>,
        #[max_length = 100]
        display_name -> Nullable<Varchar>,
        avatar_url -> Nullable<Text>,
        avatar_key -> Nullable<Text>,
        banner_url -> Nullable<Text>,
        banner_key -> Nullable<Text>,
        description -> Nullable<Text>,
        language -> Languages,
        currency -> Currencies,
        is_executor -> Bool,
        register_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    refresh_sessions (id) {
        id -> Uuid,
        user_id -> Uuid,
        family_id -> Uuid,
        token_hash -> Text,
        device_fingerprint -> Text,
        ip_address -> Inet,
        user_agent -> Text,
        expires_at -> Timestamptz,
        revoked -> Bool,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::schema::sql_types::InvitationStatus;
    use crate::schema::sql_types::TeamRoles;

    team_invitations (id) {
        id -> Uuid,
        team_id -> Uuid,
        inviter_id -> Uuid,
        invitee_id -> Uuid,
        role -> TeamRoles,
        status -> InvitationStatus,
        created_at -> Timestamptz,
        expires_at -> Nullable<Timestamptz>,
        updated_at -> Timestamptz,
    }
}

diesel::joinable!(team_members -> teams (team_id));
diesel::joinable!(team_members -> users (user_id));
diesel::joinable!(user_info_executor -> specializations (specialization));
diesel::joinable!(user_info_executor -> users (user_id));
diesel::joinable!(refresh_sessions -> users (user_id));
diesel::joinable!(team_invitations -> teams (team_id));
diesel::joinable!(team_invitations -> users (inviter_id));
//diesel::joinable!(team_invitations -> users (invitee_id));

diesel::allow_tables_to_appear_in_same_query!(
    specializations,
    team_members,
    teams,
    team_invitations,
    user_info_executor,
    users,
    refresh_sessions,
);

