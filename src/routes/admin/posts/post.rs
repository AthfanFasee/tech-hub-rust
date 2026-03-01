use actix_web::{HttpResponse, web};
use sqlx::PgPool;

use crate::{
    repository,
    routes::{PostError, PostPathParams},
};

pub async fn hard_delete_post(
    path: web::Path<PostPathParams>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, PostError> {
    let post_id = path.id;

    let deleted = repository::hard_delete_post(post_id, &pool).await?;
    if !deleted {
        return Err(PostError::NotFound);
    }

    Ok(HttpResponse::Ok().finish())
}
