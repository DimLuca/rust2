use std::{env, net::SocketAddr, str::FromStr};

use anyhow::{bail, Context, Result};
use ipnet::IpNet;

#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
    pub bind: SocketAddr,
    pub token_ttl_minutes: i64,
    pub login_limit: usize,
    pub login_window_seconds: u64,
    pub trusted_proxy_cidrs: Vec<IpNet>,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let database_url = required("DATABASE_URL")?;
        let jwt_secret = required("ARAQUARIDESK_JWT_SECRET")?;
        if jwt_secret.len() < 32 {
            bail!("ARAQUARIDESK_JWT_SECRET must contain at least 32 bytes");
        }

        Ok(Self {
            database_url,
            jwt_secret,
            bind: parse("ARAQUARIDESK_BIND", "127.0.0.1:8787")?,
            token_ttl_minutes: parse("ARAQUARIDESK_TOKEN_TTL_MINUTES", "60")?,
            login_limit: parse("ARAQUARIDESK_LOGIN_LIMIT", "5")?,
            login_window_seconds: parse("ARAQUARIDESK_LOGIN_WINDOW_SECONDS", "300")?,
            trusted_proxy_cidrs: env::var("ARAQUARIDESK_TRUSTED_PROXY_CIDRS")
                .unwrap_or_default()
                .split(',')
                .filter(|value| !value.trim().is_empty())
                .map(|value| {
                    value
                        .trim()
                        .parse()
                        .with_context(|| format!("invalid trusted proxy CIDR: {value}"))
                })
                .collect::<Result<Vec<_>>>()?,
        })
    }
}

fn required(name: &str) -> Result<String> {
    env::var(name).with_context(|| format!("missing required environment variable {name}"))
}

fn parse<T>(name: &str, default: &str) -> Result<T>
where
    T: FromStr,
    T::Err: std::error::Error + Send + Sync + 'static,
{
    env::var(name)
        .unwrap_or_else(|_| default.to_owned())
        .parse()
        .with_context(|| format!("invalid value for {name}"))
}
