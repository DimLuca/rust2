use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use tracing::error;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("authentication required")]
    Unauthorized,
    #[error("insufficient permission")]
    Forbidden,
    #[error("resource not found")]
    NotFound,
    #[error("too many authentication attempts")]
    RateLimited,
    #[error("{0}")]
    Validation(String),
    #[error("conflict")]
    Conflict,
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

#[derive(Serialize)]
struct ErrorBody {
    code: &'static str,
    message: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            Self::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "UNAUTHORIZED",
                "Usuário ou senha inválidos.".to_owned(),
            ),
            Self::Forbidden => (
                StatusCode::FORBIDDEN,
                "FORBIDDEN",
                "Acesso não autorizado.".to_owned(),
            ),
            Self::NotFound => (
                StatusCode::NOT_FOUND,
                "NOT_FOUND",
                "Recurso não encontrado.".to_owned(),
            ),
            Self::RateLimited => (
                StatusCode::TOO_MANY_REQUESTS,
                "RATE_LIMITED",
                "Muitas tentativas. Aguarde antes de tentar novamente.".to_owned(),
            ),
            Self::Validation(message) => {
                (StatusCode::BAD_REQUEST, "VALIDATION_ERROR", message)
            }
            Self::Conflict => (
                StatusCode::CONFLICT,
                "CONFLICT",
                "O registro já existe.".to_owned(),
            ),
            Self::Internal(error) => {
                error!(error = ?error, "internal API error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "INTERNAL_ERROR",
                    "Erro interno do serviço.".to_owned(),
                )
            }
        };

        (status, Json(ErrorBody { code, message })).into_response()
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(error: sqlx::Error) -> Self {
        if let sqlx::Error::Database(database) = &error {
            if database.is_unique_violation() {
                return Self::Conflict;
            }
        }
        Self::Internal(error.into())
    }
}

pub type ApiResult<T> = Result<T, ApiError>;

