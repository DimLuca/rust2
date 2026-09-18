use anyhow::{bail, Context, Result};
use araquaridesk_server::{models::Role, security::hash_password};
use clap::{Parser, Subcommand, ValueEnum};
use sqlx::{postgres::PgPoolOptions, FromRow};
use uuid::Uuid;

#[derive(Parser)]
#[command(
    name = "araquaridesk-admin",
    about = "Administração segura da AraquariDesk API"
)]
struct Cli {
    #[arg(long, env = "DATABASE_URL")]
    database_url: String,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    CreateUser {
        #[arg(long)]
        username: String,
        #[arg(long)]
        display_name: String,
        #[arg(long, value_enum, default_value = "technician")]
        role: CliRole,
    },
    ResetPassword {
        #[arg(long)]
        username: String,
    },
    DisableUser {
        #[arg(long)]
        username: String,
    },
    EnableUser {
        #[arg(long)]
        username: String,
    },
    ListUsers,
    ListAudit {
        #[arg(long, default_value_t = 100)]
        limit: i64,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum CliRole {
    Admin,
    Technician,
}

impl From<CliRole> for Role {
    fn from(role: CliRole) -> Self {
        match role {
            CliRole::Admin => Role::Admin,
            CliRole::Technician => Role::Technician,
        }
    }
}

#[derive(FromRow)]
struct ListedUser {
    id: Uuid,
    username: String,
    display_name: String,
    role: Role,
    enabled: bool,
}

type AuditRow = (
    chrono::DateTime<chrono::Utc>,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
);

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let pool = PgPoolOptions::new()
        .max_connections(3)
        .connect(&cli.database_url)
        .await
        .context("não foi possível conectar ao PostgreSQL")?;
    sqlx::migrate!().run(&pool).await?;

    match cli.command {
        Command::CreateUser {
            username,
            display_name,
            role,
        } => {
            let password = read_new_password()?;
            let password_hash = hash_password(password).await?;
            let id: Uuid = sqlx::query_scalar(
                "INSERT INTO users (username, password_hash, display_name, role) \
                 VALUES (LOWER($1), $2, $3, $4) RETURNING id",
            )
            .bind(username.trim())
            .bind(password_hash)
            .bind(display_name.trim())
            .bind(Role::from(role))
            .fetch_one(&pool)
            .await?;
            println!("Usuário criado: {id}");
        }
        Command::ResetPassword { username } => {
            let password = read_new_password()?;
            let password_hash = hash_password(password).await?;
            let mut transaction = pool.begin().await?;
            let user_id: Option<Uuid> = sqlx::query_scalar(
                "UPDATE users SET password_hash = $2, token_version = token_version + 1, \
                 updated_at = NOW() WHERE LOWER(username) = LOWER($1) RETURNING id",
            )
            .bind(username.trim())
            .bind(password_hash)
            .fetch_optional(&mut *transaction)
            .await?;
            let Some(user_id) = user_id else {
                bail!("usuário não encontrado")
            };
            sqlx::query(
                "UPDATE auth_sessions SET revoked_at = COALESCE(revoked_at, NOW()), \
                 revoke_reason = COALESCE(revoke_reason, 'PASSWORD_RESET') \
                 WHERE user_id = $1 AND revoked_at IS NULL",
            )
            .bind(user_id)
            .execute(&mut *transaction)
            .await?;
            transaction.commit().await?;
            println!("Senha alterada e sessões anteriores revogadas.");
        }
        Command::DisableUser { username } => {
            set_enabled(&pool, &username, false).await?;
            println!("Usuário desativado e sessões revogadas.");
        }
        Command::EnableUser { username } => {
            set_enabled(&pool, &username, true).await?;
            println!("Usuário ativado.");
        }
        Command::ListUsers => {
            let users = sqlx::query_as::<_, ListedUser>(
                "SELECT id, username, display_name, role, enabled FROM users ORDER BY display_name",
            )
            .fetch_all(&pool)
            .await?;
            for user in users {
                println!(
                    "{}\t{}\t{}\t{:?}\t{}",
                    user.id,
                    user.username,
                    user.display_name,
                    user.role,
                    if user.enabled { "ativo" } else { "desativado" }
                );
            }
        }
        Command::ListAudit { limit } => {
            let rows: Vec<AuditRow> = sqlx::query_as(
                "SELECT occurred_at, event_type, technician_username, client_rustdesk_id, result \
                     FROM audit_events ORDER BY occurred_at DESC LIMIT $1",
            )
            .bind(limit.clamp(1, 1000))
            .fetch_all(&pool)
            .await?;
            for (at, event, technician, client, result) in rows {
                println!(
                    "{}\t{}\t{}\t{}\t{}",
                    at.to_rfc3339(),
                    event,
                    technician.as_deref().unwrap_or("-"),
                    client.as_deref().unwrap_or("-"),
                    result.as_deref().unwrap_or("-")
                );
            }
        }
    }
    Ok(())
}

fn read_new_password() -> Result<String> {
    let password = rpassword::prompt_password("Nova senha: ")?;
    let confirmation = rpassword::prompt_password("Confirme a senha: ")?;
    if password != confirmation {
        bail!("as senhas informadas são diferentes");
    }
    Ok(password)
}

async fn set_enabled(pool: &sqlx::PgPool, username: &str, enabled: bool) -> Result<()> {
    let mut transaction = pool.begin().await?;
    let user_id: Option<Uuid> = sqlx::query_scalar(
        "UPDATE users SET enabled = $2, token_version = token_version + 1, updated_at = NOW() \
         WHERE LOWER(username) = LOWER($1) RETURNING id",
    )
    .bind(username.trim())
    .bind(enabled)
    .fetch_optional(&mut *transaction)
    .await?;
    let Some(user_id) = user_id else {
        bail!("usuário não encontrado")
    };
    if !enabled {
        sqlx::query(
            "UPDATE auth_sessions SET revoked_at = COALESCE(revoked_at, NOW()), \
             revoke_reason = COALESCE(revoke_reason, 'USER_DISABLED') \
             WHERE user_id = $1 AND revoked_at IS NULL",
        )
        .bind(user_id)
        .execute(&mut *transaction)
        .await?;
    }
    transaction.commit().await?;
    Ok(())
}
