use std::{net::SocketAddr, sync::Arc};

use anyhow::Result;
use araquaridesk_server::{
    api,
    config::Config,
    security::{hash_password, LoginLimiter},
    state::AppState,
};
use sqlx::postgres::PgPoolOptions;
use tower_http::{
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace::TraceLayer,
};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "araquaridesk_server=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    let config = Arc::new(Config::from_env()?);
    let pool = PgPoolOptions::new()
        .max_connections(20)
        .connect(&config.database_url)
        .await?;
    sqlx::migrate!().run(&pool).await?;

    let dummy_password_hash = hash_password(format!("unused-{}", uuid::Uuid::new_v4())).await?;
    let state = AppState {
        pool,
        login_limiter: Arc::new(LoginLimiter::new(
            config.login_limit,
            config.login_window_seconds,
        )),
        dummy_password_hash: Arc::from(dummy_password_hash),
        config: Arc::clone(&config),
    };
    let app = api::router(state)
        .layer(PropagateRequestIdLayer::x_request_id())
        .layer(TraceLayer::new_for_http())
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid));

    let listener = tokio::net::TcpListener::bind(config.bind).await?;
    info!(address = %config.bind, "AraquariDesk API listening");
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };
    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
