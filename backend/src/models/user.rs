use diesel::prelude::*;
use diesel::deserialize::{self, FromSql};
use diesel::pg::Pg;
use diesel::serialize::{self, ToSql, Output, IsNull};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde_json::Value as JsonValue;
use bigdecimal::BigDecimal;

use std::io::Write;

use crate::schema::*;
use crate::api_response::ApiResponse;

#[derive(utoipa::ToSchema)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[derive(diesel::AsExpression, diesel::FromSqlRow)]
#[diesel(sql_type = crate::schema::sql_types::Roles)]
pub enum UserRole {
    Unverified,
    Client,
    Executor,
    Moderator,
    Support,
    Admin,
    Devops,
    Dispatcher,
}

impl FromSql<crate::schema::sql_types::Roles, Pg> for UserRole {
    fn from_sql(bytes: diesel::pg::PgValue<'_>) -> deserialize::Result<Self> {
        let s = <String as FromSql<diesel::sql_types::Text, Pg>>::from_sql(bytes)?;
        match s.as_str() {
            "unverified" => Ok(UserRole::Unverified),
            "client" => Ok(UserRole::Client),
            "executor" => Ok(UserRole::Executor),
            "moderator" => Ok(UserRole::Moderator),
            "support" => Ok(UserRole::Support),
            "admin" => Ok(UserRole::Admin),
            "devops" => Ok(UserRole::Devops),
            "dispatcher" => Ok(UserRole::Dispatcher),
            _ => Err("Unknown role".into()),
        }
    }
}

impl ToSql<crate::schema::sql_types::Roles, Pg> for UserRole {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        match self {
            UserRole::Unverified => out.write_all(b"unverified")?,
            UserRole::Client      => out.write_all(b"client")?,
            UserRole::Executor    => out.write_all(b"executor")?,
            UserRole::Moderator   => out.write_all(b"moderator")?,
            UserRole::Support     => out.write_all(b"support")?,
            UserRole::Admin       => out.write_all(b"admin")?,
            UserRole::Devops      => out.write_all(b"devops")?,
            UserRole::Dispatcher  => out.write_all(b"dispatcher")?,
        }
        Ok(IsNull::No)
    }
}

#[derive(utoipa::ToSchema)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[derive(diesel::AsExpression, diesel::FromSqlRow)]
#[diesel(sql_type = crate::schema::sql_types::Languages)]
pub enum Language {
    Ru,
    En,
}

impl FromSql<crate::schema::sql_types::Languages, Pg> for Language {
    fn from_sql(bytes: diesel::pg::PgValue<'_>) -> deserialize::Result<Self> {
        let s = <String as FromSql<diesel::sql_types::Text, Pg>>::from_sql(bytes)?;
        match s.as_str() {
            "ru" => Ok(Language::Ru),
            "en" => Ok(Language::En),
            _ => Err("Unknown language".into()),
        }
    }
}

impl ToSql<crate::schema::sql_types::Languages, Pg> for Language {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        match self {
            Language::Ru => out.write_all(b"ru")?,
            Language::En => out.write_all(b"en")?,
        }
        Ok(IsNull::No)
    }
}

#[derive(utoipa::ToSchema)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[derive(diesel::AsExpression, diesel::FromSqlRow)]
#[diesel(sql_type = crate::schema::sql_types::Currencies)]
pub enum Currency {
    Rub,
}

impl FromSql<crate::schema::sql_types::Currencies, Pg> for Currency {
    fn from_sql(bytes: diesel::pg::PgValue<'_>) -> deserialize::Result<Self> {
        let s = <String as FromSql<diesel::sql_types::Text, Pg>>::from_sql(bytes)?;
        match s.as_str() {
            "RUB" => Ok(Currency::Rub),
            _ => Err("Unknown currency".into()),
        }
    }
}

impl ToSql<crate::schema::sql_types::Currencies, Pg> for Currency {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        match self {
            Currency::Rub => out.write_all(b"RUB")?,
        }
        Ok(IsNull::No)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[derive(diesel::AsExpression, diesel::FromSqlRow)]
#[diesel(sql_type = crate::schema::sql_types::TeamRoles)]
pub enum TeamRole {
    Owner,
    Admin,
    Manager,
    Executor,
}

impl FromSql<crate::schema::sql_types::TeamRoles, Pg> for TeamRole {
    fn from_sql(bytes: diesel::pg::PgValue<'_>) -> deserialize::Result<Self> {
        let s = <String as FromSql<diesel::sql_types::Text, Pg>>::from_sql(bytes)?;
        match s.as_str() {
            "owner" => Ok(TeamRole::Owner),
            "admin" => Ok(TeamRole::Admin),
            "manager" => Ok(TeamRole::Manager),
            "executor" => Ok(TeamRole::Executor),
            _ => Err("Unknown team role".into()),
        }
    }
}

impl ToSql<crate::schema::sql_types::TeamRoles, Pg> for TeamRole {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        match self {
            TeamRole::Owner    => out.write_all(b"owner")?,
            TeamRole::Admin    => out.write_all(b"admin")?,
            TeamRole::Manager  => out.write_all(b"manager")?,
            TeamRole::Executor => out.write_all(b"executor")?,
        }
        Ok(IsNull::No)
    }
}

#[derive(utoipa::ToSchema)]
#[derive(Queryable, Selectable, Debug, Serialize, Deserialize, Clone)]
#[diesel(table_name = users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct User {
    pub id: Uuid,
    pub email: Option<String>,
    pub role: UserRole,
    pub telegram_id: Option<i64>,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub banner_url: Option<String>,
    pub description: Option<String>,
    pub language: Language,
    pub currency: Currency,
    pub is_executor: Option<bool>,
    pub register_at: Option<DateTime<Utc>>,
}

/// Вспомогательный тип для Scalar
pub type ApiResponseUser = ApiResponse<User>;

#[derive(Insertable, Debug, Serialize, Deserialize)]
#[diesel(table_name = users)]
pub struct NewUser {
    pub email: Option<String>,
    pub role: UserRole,
    pub telegram_id: Option<i64>,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub banner_url: Option<String>,
    pub description: Option<String>,
    pub language: Language,
    pub currency: Currency,
    pub is_executor: Option<bool>,
}

// Для обновления пользователя (частично)
#[derive(AsChangeset, Debug)]
#[diesel(table_name = users)]
pub struct UpdateUser {
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub banner_url: Option<String>,
    pub description: Option<String>,
    pub language: Option<Language>,
    pub currency: Option<Currency>,
    pub is_executor: Option<bool>,
}

#[derive(Queryable, Selectable, Debug, Serialize, Deserialize)]
#[diesel(table_name = specializations)]
pub struct Specialization {
    pub id: i32,
    pub name_ru: String,
    pub name_en: String,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = specializations)]
pub struct NewSpecialization {
    pub name_ru: String,
    pub name_en: String,
}

#[derive(Queryable, Selectable, Debug, Serialize, Deserialize)]
#[diesel(table_name = user_info_executor)]
pub struct UserInfoExecutor {
    pub user_id: Uuid,
    pub specialization: Option<i32>,
    pub rating: Option<BigDecimal>,
    pub review_count: Option<i32>,
    pub completed_orders: Option<i32>,
    pub timezone: Option<String>,
    pub work_schedule: Option<JsonValue>,  // JSONB
    pub contact_rules: Option<JsonValue>,  // JSONB
}

#[derive(Insertable, AsChangeset, Debug)]
#[diesel(table_name = user_info_executor)]
pub struct NewUserInfoExecutor {
    pub user_id: Uuid,
    pub specialization: Option<i32>,
    pub rating: Option<BigDecimal>,
    pub review_count: Option<i32>,
    pub completed_orders: Option<i32>,
    pub timezone: Option<String>,
    pub work_schedule: Option<JsonValue>,
    pub contact_rules: Option<JsonValue>,
}

#[derive(Queryable, Selectable, Debug, Serialize, Deserialize)]
#[diesel(table_name = teams)]
pub struct Team {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub banner_url: Option<String>,
    pub logo_url: Option<String>,
    pub specializations: Option<Vec<Option<i32>>>,
    pub public_contacts: Option<JsonValue>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Insertable, AsChangeset, Debug)]
#[diesel(table_name = teams)]
pub struct NewTeam {
    pub name: String,
    pub description: Option<String>,
    pub banner_url: Option<String>,
    pub logo_url: Option<String>,
    pub specializations: Option<Vec<Option<i32>>>,
    pub public_contacts: Option<JsonValue>,
}

#[derive(AsChangeset, Debug)]
#[diesel(table_name = teams)]
pub struct UpdateTeam {
    pub name: Option<String>,
    pub description: Option<String>,
    pub banner_url: Option<String>,
    pub logo_url: Option<String>,
    pub specializations: Option<Vec<Option<i32>>>,
    pub public_contacts: Option<JsonValue>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Queryable, Selectable, Debug, Serialize, Deserialize)]
#[diesel(table_name = team_members)]
pub struct TeamMember {
    pub team_id: Uuid,
    pub user_id: Uuid,
    pub role: TeamRole,
    pub joined_at: Option<DateTime<Utc>>,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = team_members)]
pub struct NewTeamMember {
    pub team_id: Uuid,
    pub user_id: Uuid,
    pub role: TeamRole,
    pub joined_at: Option<DateTime<Utc>>,
}

#[derive(Queryable, Debug)]
pub struct UserWithExecutorInfo {
    pub user: User,
    pub executor_info: Option<UserInfoExecutor>,
}

#[derive(Queryable, Selectable, Identifiable, Insertable, Debug, Clone)]
#[diesel(table_name = refresh_sessions)]
pub struct RefreshSession {
    pub id: Uuid,
    pub user_id: Uuid,
    pub family_id: Uuid,
    pub token_hash: String,
    pub device_fingerprint: String,
    pub ip_address: ipnetwork::IpNetwork,
    pub user_agent: String,
    pub expires_at: chrono::NaiveDateTime,
    pub revoked: bool,
    pub created_at: chrono::NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = refresh_sessions)]
pub struct NewRefreshSession {
    pub user_id: Uuid,
    pub family_id: Uuid,
    pub token_hash: String,
    pub device_fingerprint: String,
    pub ip_address: ipnetwork::IpNetwork,
    pub user_agent: String,
    pub expires_at: chrono::NaiveDateTime,
}

