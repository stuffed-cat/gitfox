use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;


/// Namespace represents either a user or a group
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Namespace {
    pub id: i64,
    pub name: String,
    pub path: String,
    pub description: Option<String>,
    pub avatar_url: Option<String>,
    pub namespace_type: NamespaceType,
    pub parent_id: Option<i64>,  // For subgroups
    pub visibility: NamespaceVisibility,
    pub owner_id: Option<i64>,   // Only for groups, users own themselves
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "namespace_type", rename_all = "lowercase")]
pub enum NamespaceType {
    User,
    Group,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "namespace_visibility", rename_all = "lowercase")]
pub enum NamespaceVisibility {
    Public,
    Private,
    Internal,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Group {
    pub id: i64,
    pub namespace_id: i64,
    pub name: String,
    pub path: String,
    pub description: Option<String>,
    pub avatar_url: Option<String>,
    pub visibility: NamespaceVisibility,
    pub parent_id: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GroupMember {
    pub id: i64,
    pub group_id: i64,
    pub user_id: i64,
    pub access_level: i32,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, PartialOrd)]
#[serde(rename_all = "lowercase")]
#[repr(i32)]
pub enum AccessLevel {
    Guest = 10,
    Reporter = 20,
    Developer = 30,
    Maintainer = 40,
    Owner = 50,
}

#[derive(Debug, Deserialize)]
pub struct CreateGroupRequest {
    pub name: String,
    pub path: String,
    pub description: Option<String>,
    pub visibility: Option<NamespaceVisibility>,
    pub parent_id: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateGroupRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub visibility: Option<NamespaceVisibility>,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AddGroupMemberRequest {
    pub user_id: i64,
    pub access_level: AccessLevel,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct GroupWithDetails {
    pub group: Group,
    pub full_path: String,
    pub parent: Option<Box<GroupWithDetails>>,
    pub projects_count: i64,
    pub members_count: i64,
    pub subgroups_count: i64,
}

#[derive(Debug, Serialize, FromRow)]
pub struct NamespaceInfo {
    pub id: i64,
    pub name: String,
    pub path: String,
    pub full_path: String,
    pub namespace_type: NamespaceType,
    pub avatar_url: Option<String>,
}
