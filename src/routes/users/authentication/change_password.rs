use std::fmt::{self, Debug, Formatter};

use actix_web::{HttpResponse, ResponseError, http::StatusCode, web};
use sqlx::PgPool;

use crate::{
    authentication,
    authentication::{AuthError, Credentials, UserId},
    domain::ChangePasswordData,
    repository, utils,
};

#[derive(thiserror::Error)]
pub enum ChangePasswordError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error("Invalid request: {0}")]
    BadRequest(String),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl Debug for ChangePasswordError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        utils::error_chain_fmt(self, f)
    }
}

impl ResponseError for ChangePasswordError {
    fn error_response(&self) -> HttpResponse {
        let status_code = match self {
            ChangePasswordError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ChangePasswordError::AuthError(_) => StatusCode::UNAUTHORIZED,
            ChangePasswordError::BadRequest(_) => StatusCode::BAD_REQUEST,
        };

        utils::build_error_response(status_code, self.to_string())
    }
}
#[tracing::instrument(
    skip_all,
    fields(user_id=%&*user_id)
)]
pub async fn change_password(
    payload: web::Json<ChangePasswordData>,
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, ChangePasswordError> {
    let user_id = user_id.into_inner();
    let username = repository::get_username(*user_id, &pool).await?;

    let (current_password, new_password) = payload
        .0
        .try_into()
        .map_err(ChangePasswordError::BadRequest)?;

    let credentials = Credentials {
        user_name: username,
        password: current_password.into_secret(),
    };

    if let Err(e) = authentication::validate_credentials(credentials, &pool).await {
        return match e {
            AuthError::InvalidCredentials(_) => Err(ChangePasswordError::AuthError(e.into())),
            AuthError::UnexpectedError(_) => Err(ChangePasswordError::UnexpectedError(e.into())),
        };
    }

    authentication::change_password(*user_id, new_password.into_secret(), &pool).await?;

    Ok(HttpResponse::Ok().finish())
}
