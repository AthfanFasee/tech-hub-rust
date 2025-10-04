use actix_web::HttpResponse;
use actix_web::http::StatusCode;
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
    e: &(dyn std::error::Error),
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    // Top-level: use Display to avoid recursion
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
