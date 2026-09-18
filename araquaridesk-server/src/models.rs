use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "user_role", rename_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Role {
    Admin,
    Technician,
}

#[derive(Clone, Debug, FromRow)]
pub struct UserRecord {
    pub id: Uuid,
    pub username: String,
    pub password_hash: String,
    pub display_name: String,
    pub role: Role,
    pub enabled: bool,
    pub token_version: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_login_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PublicUser {
    pub id: Uuid,
    pub username: String,
    pub display_name: String,
    pub role: Role,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_login_at: Option<DateTime<Utc>>,
}

impl From<UserRecord> for PublicUser {
    fn from(user: UserRecord) -> Self {
        Self {
            id: user.id,
            username: user.username,
            display_name: user.display_name,
            role: user.role,
            enabled: user.enabled,
            created_at: user.created_at,
            updated_at: user.updated_at,
            last_login_at: user.last_login_at,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
    pub device: Option<DeviceIdentity>,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub token_type: &'static str,
    pub expires_at: DateTime<Utc>,
    pub user: PublicUser,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct DeviceIdentity {
    pub device_id: Option<String>,
    pub hostname: Option<String>,
    pub os_username: Option<String>,
    pub local_ip: Option<String>,
    pub rustdesk_id: Option<String>,
}

#[derive(Clone, Debug)]
pub struct AuthContext {
    pub session_id: Uuid,
    pub user: PublicUser,
}

#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub password: String,
    pub display_name: String,
    pub role: Role,
}

#[derive(Debug, Deserialize)]
pub struct UpdateUserRequest {
    pub display_name: Option<String>,
    pub role: Option<Role>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct ResetPasswordRequest {
    pub password: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AuditEventType {
    LoginSuccess,
    LoginFailed,
    Logout,
    TiModeStarted,
    TiModeEnded,
    RemoteConnectionRequested,
    RemoteConnectionStarted,
    RemoteConnectionEnded,
    RemoteConnectionFailed,
    RemoteConnectionRejected,
}

impl AuditEventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::LoginSuccess => "LOGIN_SUCCESS",
            Self::LoginFailed => "LOGIN_FAILED",
            Self::Logout => "LOGOUT",
            Self::TiModeStarted => "TI_MODE_STARTED",
            Self::TiModeEnded => "TI_MODE_ENDED",
            Self::RemoteConnectionRequested => "REMOTE_CONNECTION_REQUESTED",
            Self::RemoteConnectionStarted => "REMOTE_CONNECTION_STARTED",
            Self::RemoteConnectionEnded => "REMOTE_CONNECTION_ENDED",
            Self::RemoteConnectionFailed => "REMOTE_CONNECTION_FAILED",
            Self::RemoteConnectionRejected => "REMOTE_CONNECTION_REJECTED",
        }
    }

    pub fn is_client_reportable(&self) -> bool {
        !matches!(self, Self::LoginSuccess | Self::LoginFailed | Self::Logout)
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct AuditEventRequest {
    pub event_type: AuditEventType,
    pub support_session_id: Option<Uuid>,
    pub technician_device: Option<DeviceIdentity>,
    pub client_device: Option<DeviceIdentity>,
    pub result: Option<String>,
    pub duration_seconds: Option<i64>,
    #[serde(default)]
    pub metadata: serde_json::Value,
}

#[derive(Debug, Serialize, FromRow)]
pub struct AuditEventView {
    pub id: Uuid,
    pub event_type: String,
    pub occurred_at: DateTime<Utc>,
    pub support_session_id: Option<Uuid>,
    pub technician_username: Option<String>,
    pub technician_display_name: Option<String>,
    pub technician_role: Option<String>,
    pub technician_device_id: Option<String>,
    pub technician_hostname: Option<String>,
    pub technician_local_ip: Option<String>,
    pub technician_source_ip: Option<String>,
    pub client_rustdesk_id: Option<String>,
    pub client_device_id: Option<String>,
    pub client_hostname: Option<String>,
    pub client_os_username: Option<String>,
    pub client_local_ip: Option<String>,
    pub result: Option<String>,
    pub duration_seconds: Option<i64>,
    pub metadata: serde_json::Value,
}
