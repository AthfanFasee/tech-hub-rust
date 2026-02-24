use actix_web::{HttpResponse, error, http::StatusCode};
use rand::{Rng, distributions::Alphanumeric};
use serde::Serialize;

#[derive(Serialize)]
pub struct ErrorResponse {
    pub code: u16,
    pub message: String,
}

pub fn build_error_response(status_code: StatusCode, message: String) -> HttpResponse {
    let error_response = ErrorResponse {
        code: status_code.as_u16(),
        message,
    };
    HttpResponse::build(status_code).json(error_response)
}

pub fn error_chain_fmt(
    e: &dyn std::error::Error,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    writeln!(f, "{e}")?;

    let mut current = e.source();
    while let Some(cause) = current {
        // For causes: use Debug if caller asked for `:#?` (`tracing::debug!("{:#?}", err)`), else Display (`tracing::error!("{:?}", err)`)
        if f.alternate() {
            writeln!(f, "Caused by:\n\t{cause:?}")?;
        } else {
            writeln!(f, "Caused by:\n\t{cause}")?;
        }
        current = cause.source();
    }
    Ok(())
}

pub fn generate_token() -> String {
    let mut rng = rand::thread_rng();
    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
}

// Generic error helper that wraps any error into an appropriate Actix error while preserving root causes
pub fn app_error<T>(status: StatusCode, e: T) -> actix_web::Error
where
    T: std::fmt::Debug + std::fmt::Display + 'static,
{
    match status {
        StatusCode::BAD_REQUEST => error::ErrorBadRequest(e),
        StatusCode::UNAUTHORIZED => error::ErrorUnauthorized(e),
        StatusCode::FORBIDDEN => error::ErrorForbidden(e),
        StatusCode::INTERNAL_SERVER_ERROR => error::ErrorInternalServerError(e),
        _ => error::ErrorInternalServerError(e),
    }
}
