use std::sync::Arc;

use sqlx::PgPool;

use crate::{config::Config, security::LoginLimiter};

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub config: Arc<Config>,
    pub login_limiter: Arc<LoginLimiter>,
    pub dummy_password_hash: Arc<str>,
}

