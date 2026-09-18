use std::{
    collections::{HashMap, VecDeque},
    sync::Mutex,
    time::{Duration, Instant},
};

use anyhow::{anyhow, Context, Result};
use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Algorithm, Argon2, Params, Version,
};
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use rand_core::OsRng;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::Role;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Claims {
    pub sub: Uuid,
    pub jti: Uuid,
    pub role: Role,
    pub ver: i32,
    pub iat: usize,
    pub exp: usize,
}

pub async fn hash_password(password: String) -> Result<String> {
    validate_password(&password)?;
    tokio::task::spawn_blocking(move || {
        let params = Params::new(19_456, 2, 1, None).context("invalid Argon2 parameters")?;
        let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
        let salt = SaltString::generate(&mut OsRng);
        argon2
            .hash_password(password.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .map_err(|error| anyhow!("failed to hash password: {error}"))
    })
    .await
    .context("password hashing task failed")?
}

pub async fn verify_password(password: String, encoded: String) -> Result<bool> {
    tokio::task::spawn_blocking(move || {
        let parsed = PasswordHash::new(&encoded)
            .map_err(|error| anyhow!("invalid stored password hash: {error}"))?;
        Ok(Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok())
    })
    .await
    .context("password verification task failed")?
}

pub fn validate_password(password: &str) -> Result<()> {
    if password.len() < 12 || password.len() > 256 {
        return Err(anyhow!(
            "password must contain between 12 and 256 characters"
        ));
    }
    Ok(())
}

pub fn create_token(
    secret: &str,
    user_id: Uuid,
    role: Role,
    token_version: i32,
    ttl_minutes: i64,
) -> Result<(String, Uuid, DateTime<Utc>)> {
    let now = Utc::now();
    let expires_at = now + ChronoDuration::minutes(ttl_minutes);
    let session_id = Uuid::new_v4();
    let claims = Claims {
        sub: user_id,
        jti: session_id,
        role,
        ver: token_version,
        iat: now.timestamp() as usize,
        exp: expires_at.timestamp() as usize,
    };
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )?;
    Ok((token, session_id, expires_at))
}

pub fn decode_token(secret: &str, token: &str) -> Result<Claims> {
    let mut validation = Validation::default();
    validation.validate_exp = true;
    validation.set_required_spec_claims(&["sub", "jti", "iat", "exp"]);
    Ok(decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )?
    .claims)
}

#[derive(Debug)]
pub struct LoginLimiter {
    attempts: Mutex<HashMap<String, VecDeque<Instant>>>,
    max_attempts: usize,
    window: Duration,
}

impl LoginLimiter {
    pub fn new(max_attempts: usize, window_seconds: u64) -> Self {
        Self {
            attempts: Mutex::new(HashMap::new()),
            max_attempts,
            window: Duration::from_secs(window_seconds),
        }
    }

    pub fn check(&self, key: &str) -> bool {
        let now = Instant::now();
        let mut attempts = self.attempts.lock().expect("login limiter poisoned");
        let entries = attempts.entry(key.to_owned()).or_default();
        while entries
            .front()
            .is_some_and(|at| now.duration_since(*at) >= self.window)
        {
            entries.pop_front();
        }
        entries.len() < self.max_attempts
    }

    pub fn record_failure(&self, key: &str) {
        let mut attempts = self.attempts.lock().expect("login limiter poisoned");
        attempts
            .entry(key.to_owned())
            .or_default()
            .push_back(Instant::now());
    }

    pub fn clear(&self, key: &str) {
        self.attempts
            .lock()
            .expect("login limiter poisoned")
            .remove(key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn argon2id_hash_round_trip() {
        let hash = hash_password("a-strong-test-password".to_owned())
            .await
            .unwrap();
        assert!(hash.starts_with("$argon2id$"));
        assert!(
            verify_password("a-strong-test-password".to_owned(), hash.clone())
                .await
                .unwrap()
        );
        assert!(!verify_password("incorrect-password".to_owned(), hash)
            .await
            .unwrap());
    }

    #[test]
    fn token_round_trip_and_role() {
        let user_id = Uuid::new_v4();
        let secret = "this-is-a-test-secret-with-at-least-32-bytes";
        let (token, session_id, _) =
            create_token(secret, user_id, Role::Technician, 3, 60).unwrap();
        let claims = decode_token(secret, &token).unwrap();
        assert_eq!(claims.sub, user_id);
        assert_eq!(claims.jti, session_id);
        assert_eq!(claims.role, Role::Technician);
        assert_eq!(claims.ver, 3);
    }

    #[test]
    fn limiter_blocks_after_configured_failures() {
        let limiter = LoginLimiter::new(2, 60);
        assert!(limiter.check("client"));
        limiter.record_failure("client");
        assert!(limiter.check("client"));
        limiter.record_failure("client");
        assert!(!limiter.check("client"));
        limiter.clear("client");
        assert!(limiter.check("client"));
    }
}
