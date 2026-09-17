use std::{
    net::{IpAddr, SocketAddr},
    str::FromStr,
};

use anyhow::Context;
use axum::{
    extract::{ConnectInfo, Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    routing::{get, patch, post},
    Json, Router,
};
use chrono::Utc;
use ipnet::IpNet;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    models::{
        AuditEventRequest, AuditEventView, AuthContext, CreateUserRequest, LoginRequest,
        LoginResponse, PublicUser, ResetPasswordRequest, Role, UpdateUserRequest, UserRecord,
    },
    security::{create_token, decode_token, hash_password, verify_password},
    state::AppState,
};

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/v1/auth/login", post(login))
        .route("/api/v1/auth/logout", post(logout))
        .route("/api/v1/auth/me", get(me))
        .route(
            "/api/v1/audit/events",
            post(create_audit_event).get(list_audit_events),
        )
        .route("/api/v1/users", get(list_users).post(create_user))
        .route("/api/v1/users/:id", patch(update_user))
        .route("/api/v1/users/:id/reset-password", post(reset_password))
        .route("/api/v1/users/:id/disable", post(disable_user))
        .with_state(state)
}

async fn health(State(state): State<AppState>) -> ApiResult<Json<Value>> {
    sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&state.pool)
        .await?;
    Ok(Json(json!({"status": "ok"})))
}

async fn login(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(request): Json<LoginRequest>,
) -> ApiResult<Json<LoginResponse>> {
    let username = normalize_username(&request.username)?;
    if request.password.is_empty() || request.password.len() > 256 {
        return Err(ApiError::Unauthorized);
    }
    let source_ip = resolve_source_ip(peer.ip(), &headers, &state.config.trusted_proxy_cidrs);
    let limiter_key = format!("{source_ip}|{username}");
    if !state.login_limiter.check(&limiter_key) {
        record_login_failure(
            &state.pool,
            &username,
            source_ip,
            "RATE_LIMITED",
            request.device.as_ref(),
        )
        .await?;
        return Err(ApiError::RateLimited);
    }

    let user = find_user_by_username(&state.pool, &username).await?;
    let encoded = user
        .as_ref()
        .map(|value| value.password_hash.clone())
        .unwrap_or_else(|| state.dummy_password_hash.to_string());
    let password_valid = verify_password(request.password, encoded)
        .await
        .context("password verification failed")?;

    let Some(user) = user.filter(|value| value.enabled && password_valid) else {
        state.login_limiter.record_failure(&limiter_key);
        record_login_failure(
            &state.pool,
            &username,
            source_ip,
            "INVALID_CREDENTIALS",
            request.device.as_ref(),
        )
        .await?;
        return Err(ApiError::Unauthorized);
    };

    state.login_limiter.clear(&limiter_key);
    let (access_token, session_id, expires_at) = create_token(
        &state.config.jwt_secret,
        user.id,
        user.role,
        user.token_version,
        state.config.token_ttl_minutes,
    )
    .context("token creation failed")?;

    let user_agent = headers
        .get(header::USER_AGENT)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.chars().take(512).collect::<String>());

    let mut transaction = state.pool.begin().await?;
    sqlx::query(
        "INSERT INTO auth_sessions (id, user_id, token_version, source_ip, user_agent, expires_at) \
         VALUES ($1, $2, $3, $4::inet, $5, $6)",
    )
    .bind(session_id)
    .bind(user.id)
    .bind(user.token_version)
    .bind(source_ip.to_string())
    .bind(user_agent)
    .bind(expires_at)
    .execute(&mut *transaction)
    .await?;
    sqlx::query("UPDATE users SET last_login_at = NOW(), updated_at = NOW() WHERE id = $1")
        .bind(user.id)
        .execute(&mut *transaction)
        .await?;
    insert_login_success(
        &mut transaction,
        &user,
        session_id,
        source_ip,
        request.device.as_ref(),
    )
    .await?;
    transaction.commit().await?;

    let mut public_user = PublicUser::from(user);
    public_user.last_login_at = Some(Utc::now());
    Ok(Json(LoginResponse {
        access_token,
        token_type: "Bearer",
        expires_at,
        user: public_user,
    }))
}

async fn logout(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> ApiResult<StatusCode> {
    let auth = authenticate(&state, &headers).await?;
    let source_ip = resolve_source_ip(peer.ip(), &headers, &state.config.trusted_proxy_cidrs);
    let mut transaction = state.pool.begin().await?;
    sqlx::query(
        "UPDATE auth_sessions SET revoked_at = COALESCE(revoked_at, NOW()), \
         revoke_reason = COALESCE(revoke_reason, 'LOGOUT') WHERE id = $1",
    )
    .bind(auth.session_id)
    .execute(&mut *transaction)
    .await?;
    insert_server_audit(
        &mut transaction,
        "LOGOUT",
        &auth,
        source_ip,
        None,
        Some("SUCCESS"),
    )
    .await?;
    transaction.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn me(State(state): State<AppState>, headers: HeaderMap) -> ApiResult<Json<PublicUser>> {
    Ok(Json(authenticate(&state, &headers).await?.user))
}

async fn create_audit_event(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(request): Json<AuditEventRequest>,
) -> ApiResult<(StatusCode, Json<Value>)> {
    let auth = authenticate(&state, &headers).await?;
    validate_audit_request(&request)?;
    let source_ip = resolve_source_ip(peer.ip(), &headers, &state.config.trusted_proxy_cidrs);
    let technician = request.technician_device.as_ref();
    let client = request.client_device.as_ref();

    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO audit_events (event_type, support_session_id, auth_session_id, \
         technician_user_id, technician_username, technician_display_name, technician_role, \
         technician_device_id, technician_hostname, technician_local_ip, technician_source_ip, \
         client_rustdesk_id, client_device_id, client_hostname, client_os_username, client_local_ip, \
         result, duration_seconds, metadata) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10::inet, $11::inet, \
                 $12, $13, $14, $15, $16::inet, $17, $18, $19) RETURNING id",
    )
    .bind(request.event_type.as_str())
    .bind(request.support_session_id)
    .bind(auth.session_id)
    .bind(auth.user.id)
    .bind(&auth.user.username)
    .bind(&auth.user.display_name)
    .bind(role_name(auth.user.role))
    .bind(technician.and_then(|d| d.device_id.as_deref()))
    .bind(technician.and_then(|d| d.hostname.as_deref()))
    .bind(normalize_optional_ip(technician.and_then(|d| d.local_ip.as_deref()))?)
    .bind(source_ip.to_string())
    .bind(client.and_then(|d| d.rustdesk_id.as_deref()))
    .bind(client.and_then(|d| d.device_id.as_deref()))
    .bind(client.and_then(|d| d.hostname.as_deref()))
    .bind(client.and_then(|d| d.os_username.as_deref()))
    .bind(normalize_optional_ip(client.and_then(|d| d.local_ip.as_deref()))?)
    .bind(request.result.as_deref())
    .bind(request.duration_seconds)
    .bind(request.metadata)
    .fetch_one(&state.pool)
    .await?;

    Ok((StatusCode::CREATED, Json(json!({"id": id}))))
}

#[derive(Deserialize)]
struct AuditQuery {
    limit: Option<i64>,
    offset: Option<i64>,
    support_session_id: Option<Uuid>,
}

async fn list_audit_events(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<AuditQuery>,
) -> ApiResult<Json<Vec<AuditEventView>>> {
    require_admin(authenticate(&state, &headers).await?)?;
    let limit = query.limit.unwrap_or(100).clamp(1, 500);
    let offset = query.offset.unwrap_or(0).max(0);
    let events = sqlx::query_as::<_, AuditEventView>(
        "SELECT id, event_type, occurred_at, support_session_id, technician_username, \
         technician_display_name, technician_role, technician_device_id, technician_hostname, \
         host(technician_local_ip) AS technician_local_ip, \
         host(technician_source_ip) AS technician_source_ip, client_rustdesk_id, client_device_id, \
         client_hostname, client_os_username, host(client_local_ip) AS client_local_ip, result, \
         duration_seconds, metadata FROM audit_events \
         WHERE ($1::uuid IS NULL OR support_session_id = $1) \
         ORDER BY occurred_at DESC LIMIT $2 OFFSET $3",
    )
    .bind(query.support_session_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(events))
}

async fn list_users(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<PublicUser>>> {
    require_admin(authenticate(&state, &headers).await?)?;
    let users = sqlx::query_as::<_, UserRecord>(
        "SELECT id, username, password_hash, display_name, role, enabled, token_version, \
         created_at, updated_at, last_login_at FROM users ORDER BY display_name, username",
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(users.into_iter().map(PublicUser::from).collect()))
}

async fn create_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<CreateUserRequest>,
) -> ApiResult<(StatusCode, Json<PublicUser>)> {
    require_admin(authenticate(&state, &headers).await?)?;
    let username = normalize_username(&request.username)?;
    let display_name = validate_display_name(&request.display_name)?;
    let password_hash = hash_password(request.password)
        .await
        .map_err(|error| ApiError::Validation(error.to_string()))?;
    let user = sqlx::query_as::<_, UserRecord>(
        "INSERT INTO users (username, password_hash, display_name, role) VALUES ($1, $2, $3, $4) \
         RETURNING id, username, password_hash, display_name, role, enabled, token_version, \
         created_at, updated_at, last_login_at",
    )
    .bind(username)
    .bind(password_hash)
    .bind(display_name)
    .bind(request.role)
    .fetch_one(&state.pool)
    .await?;
    Ok((StatusCode::CREATED, Json(PublicUser::from(user))))
}

async fn update_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateUserRequest>,
) -> ApiResult<Json<PublicUser>> {
    let actor = require_admin(authenticate(&state, &headers).await?)?;
    if actor.user.id == id
        && (request.enabled == Some(false) || request.role == Some(Role::Technician))
    {
        return Err(ApiError::Validation(
            "não é permitido desativar ou remover o próprio papel ADMIN".to_owned(),
        ));
    }
    let display_name = request
        .display_name
        .as_deref()
        .map(validate_display_name)
        .transpose()?;
    let user = sqlx::query_as::<_, UserRecord>(
        "UPDATE users SET display_name = COALESCE($2, display_name), role = COALESCE($3, role), \
         enabled = COALESCE($4, enabled), token_version = CASE WHEN $4 = FALSE THEN token_version + 1 ELSE token_version END, \
         updated_at = NOW() WHERE id = $1 RETURNING id, username, password_hash, display_name, role, \
         enabled, token_version, created_at, updated_at, last_login_at",
    )
    .bind(id)
    .bind(display_name)
    .bind(request.role)
    .bind(request.enabled)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(ApiError::NotFound)?;
    if request.enabled == Some(false) {
        revoke_user_sessions(&state.pool, id, "USER_DISABLED").await?;
    }
    Ok(Json(PublicUser::from(user)))
}

async fn reset_password(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(request): Json<ResetPasswordRequest>,
) -> ApiResult<StatusCode> {
    require_admin(authenticate(&state, &headers).await?)?;
    let password_hash = hash_password(request.password)
        .await
        .map_err(|error| ApiError::Validation(error.to_string()))?;
    let result = sqlx::query(
        "UPDATE users SET password_hash = $2, token_version = token_version + 1, updated_at = NOW() WHERE id = $1",
    )
    .bind(id)
    .bind(password_hash)
    .execute(&state.pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound);
    }
    revoke_user_sessions(&state.pool, id, "PASSWORD_RESET").await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn disable_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    let actor = require_admin(authenticate(&state, &headers).await?)?;
    if actor.user.id == id {
        return Err(ApiError::Validation(
            "não é permitido desativar o próprio usuário".to_owned(),
        ));
    }
    let result = sqlx::query(
        "UPDATE users SET enabled = FALSE, token_version = token_version + 1, updated_at = NOW() WHERE id = $1",
    )
    .bind(id)
    .execute(&state.pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound);
    }
    revoke_user_sessions(&state.pool, id, "USER_DISABLED").await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn authenticate(state: &AppState, headers: &HeaderMap) -> ApiResult<AuthContext> {
    let token = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .ok_or(ApiError::Unauthorized)?;
    let claims =
        decode_token(&state.config.jwt_secret, token).map_err(|_| ApiError::Unauthorized)?;
    let user = sqlx::query_as::<_, UserRecord>(
        "SELECT u.id, u.username, u.password_hash, u.display_name, u.role, u.enabled, \
         u.token_version, u.created_at, u.updated_at, u.last_login_at \
         FROM auth_sessions s JOIN users u ON u.id = s.user_id \
         WHERE s.id = $1 AND s.user_id = $2 AND s.revoked_at IS NULL AND s.expires_at > NOW() \
           AND s.token_version = u.token_version AND u.enabled = TRUE",
    )
    .bind(claims.jti)
    .bind(claims.sub)
    .fetch_optional(&state.pool)
    .await?
    .filter(|user| user.token_version == claims.ver && user.role == claims.role)
    .ok_or(ApiError::Unauthorized)?;
    Ok(AuthContext {
        session_id: claims.jti,
        user: PublicUser::from(user),
    })
}

fn require_admin(auth: AuthContext) -> ApiResult<AuthContext> {
    if auth.user.role != Role::Admin {
        return Err(ApiError::Forbidden);
    }
    Ok(auth)
}

async fn find_user_by_username(
    pool: &PgPool,
    username: &str,
) -> Result<Option<UserRecord>, sqlx::Error> {
    sqlx::query_as::<_, UserRecord>(
        "SELECT id, username, password_hash, display_name, role, enabled, token_version, \
         created_at, updated_at, last_login_at FROM users WHERE LOWER(username) = LOWER($1)",
    )
    .bind(username)
    .fetch_optional(pool)
    .await
}

async fn revoke_user_sessions(
    pool: &PgPool,
    user_id: Uuid,
    reason: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE auth_sessions SET revoked_at = COALESCE(revoked_at, NOW()), \
         revoke_reason = COALESCE(revoke_reason, $2) WHERE user_id = $1 AND revoked_at IS NULL",
    )
    .bind(user_id)
    .bind(reason)
    .execute(pool)
    .await?;
    Ok(())
}

async fn record_login_failure(
    pool: &PgPool,
    username: &str,
    source_ip: IpAddr,
    result: &str,
    device: Option<&crate::models::DeviceIdentity>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO audit_events (event_type, technician_username, technician_device_id, \
         technician_hostname, technician_local_ip, technician_source_ip, result) \
         VALUES ('LOGIN_FAILED', $1, $2, $3, $4::inet, $5::inet, $6)",
    )
    .bind(username)
    .bind(device.and_then(|value| value.device_id.as_deref()))
    .bind(device.and_then(|value| value.hostname.as_deref()))
    .bind(device.and_then(|value| value.local_ip.as_deref()))
    .bind(source_ip.to_string())
    .bind(result)
    .execute(pool)
    .await?;
    Ok(())
}

async fn insert_login_success(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user: &UserRecord,
    session_id: Uuid,
    source_ip: IpAddr,
    device: Option<&crate::models::DeviceIdentity>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO audit_events (event_type, auth_session_id, technician_user_id, technician_username, \
         technician_display_name, technician_role, technician_device_id, technician_hostname, \
         technician_local_ip, technician_source_ip, result) \
         VALUES ('LOGIN_SUCCESS', $1, $2, $3, $4, $5, $6, $7, $8::inet, $9::inet, 'SUCCESS')",
    )
    .bind(session_id)
    .bind(user.id)
    .bind(&user.username)
    .bind(&user.display_name)
    .bind(role_name(user.role))
    .bind(device.and_then(|value| value.device_id.as_deref()))
    .bind(device.and_then(|value| value.hostname.as_deref()))
    .bind(device.and_then(|value| value.local_ip.as_deref()))
    .bind(source_ip.to_string())
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

async fn insert_server_audit(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    event_type: &str,
    auth: &AuthContext,
    source_ip: IpAddr,
    support_session_id: Option<Uuid>,
    result: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO audit_events (event_type, support_session_id, auth_session_id, technician_user_id, \
         technician_username, technician_display_name, technician_role, technician_source_ip, result) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8::inet, $9)",
    )
    .bind(event_type)
    .bind(support_session_id)
    .bind(auth.session_id)
    .bind(auth.user.id)
    .bind(&auth.user.username)
    .bind(&auth.user.display_name)
    .bind(role_name(auth.user.role))
    .bind(source_ip.to_string())
    .bind(result)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

fn resolve_source_ip(peer: IpAddr, headers: &HeaderMap, trusted: &[IpNet]) -> IpAddr {
    if !trusted.iter().any(|network| network.contains(&peer)) {
        return peer;
    }
    let forwarded = headers
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
        .into_iter()
        .flat_map(|value| value.split(','))
        .filter_map(|value| IpAddr::from_str(value.trim()).ok())
        .collect::<Vec<_>>();
    forwarded
        .into_iter()
        .rev()
        .find(|candidate| !trusted.iter().any(|network| network.contains(candidate)))
        .unwrap_or(peer)
}

fn normalize_username(username: &str) -> ApiResult<String> {
    let username = username.trim().to_lowercase();
    let valid = (3..=64).contains(&username.len())
        && username.chars().all(|character| {
            character.is_ascii_lowercase()
                || character.is_ascii_digit()
                || ".-_".contains(character)
        });
    if !valid {
        return Err(ApiError::Validation("usuário inválido".to_owned()));
    }
    Ok(username)
}

fn validate_display_name(value: &str) -> ApiResult<String> {
    let value = value.trim();
    if value.len() < 2 || value.len() > 120 {
        return Err(ApiError::Validation("nome de exibição inválido".to_owned()));
    }
    Ok(value.to_owned())
}

fn normalize_optional_ip(value: Option<&str>) -> ApiResult<Option<String>> {
    value
        .map(|value| value.parse::<IpAddr>().map(|ip| ip.to_string()))
        .transpose()
        .map_err(|_| ApiError::Validation("endereço IP local inválido".to_owned()))
}

fn validate_audit_request(request: &AuditEventRequest) -> ApiResult<()> {
    if !request.event_type.is_client_reportable() {
        return Err(ApiError::Validation(
            "evento reservado para registro interno do servidor".to_owned(),
        ));
    }
    if request.duration_seconds.is_some_and(|value| value < 0) {
        return Err(ApiError::Validation("duração inválida".to_owned()));
    }
    validate_device_identity(request.technician_device.as_ref())?;
    validate_device_identity(request.client_device.as_ref())?;
    if request.result.as_ref().is_some_and(|value| value.len() > 128) {
        return Err(ApiError::Validation("resultado inválido".to_owned()));
    }
    let serialized = serde_json::to_vec(&request.metadata)
        .map_err(|_| ApiError::Validation("metadata inválida".to_owned()))?;
    if serialized.len() > 16 * 1024 || contains_sensitive_key(&request.metadata) {
        return Err(ApiError::Validation(
            "metadata contém dados não permitidos ou excede o limite".to_owned(),
        ));
    }
    Ok(())
}

fn validate_device_identity(device: Option<&crate::models::DeviceIdentity>) -> ApiResult<()> {
    let Some(device) = device else {
        return Ok(());
    };
    let fields = [
        (device.device_id.as_deref(), 128),
        (device.hostname.as_deref(), 255),
        (device.os_username.as_deref(), 255),
        (device.rustdesk_id.as_deref(), 64),
    ];
    if fields
        .into_iter()
        .any(|(value, limit)| value.is_some_and(|value| value.len() > limit))
    {
        return Err(ApiError::Validation(
            "identificação do dispositivo excede o limite".to_owned(),
        ));
    }
    normalize_optional_ip(device.local_ip.as_deref())?;
    Ok(())
}

fn contains_sensitive_key(value: &Value) -> bool {
    const DENIED: &[&str] = &[
        "password",
        "senha",
        "token",
        "clipboard",
        "screen",
        "content",
        "file_content",
        "keystrokes",
    ];
    match value {
        Value::Object(map) => map.iter().any(|(key, value)| {
            DENIED
                .iter()
                .any(|denied| key.to_lowercase().contains(denied))
                || contains_sensitive_key(value)
        }),
        Value::Array(values) => values.iter().any(contains_sensitive_key),
        _ => false,
    }
}

fn role_name(role: Role) -> &'static str {
    match role {
        Role::Admin => "ADMIN",
        Role::Technician => "TECHNICIAN",
    }
}

#[derive(Serialize)]
struct _NeverExposePasswordHash;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn untrusted_peer_cannot_spoof_forwarded_ip() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "10.0.0.5".parse().unwrap());
        let trusted = vec!["127.0.0.1/32".parse().unwrap()];
        assert_eq!(
            resolve_source_ip("203.0.113.9".parse().unwrap(), &headers, &trusted),
            "203.0.113.9".parse::<IpAddr>().unwrap()
        );
    }

    #[test]
    fn sensitive_audit_metadata_is_rejected() {
        assert!(contains_sensitive_key(
            &json!({"nested": {"password": "secret"}})
        ));
        assert!(!contains_sensitive_key(
            &json!({"connection_type": "remote"})
        ));
    }
}
