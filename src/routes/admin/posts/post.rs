use crate::authentication::UserId;
use crate::routes::{PostError, PostPathParams};
use actix_web::{HttpResponse, web};
use anyhow::Context;
use sqlx::PgPool;

#[tracing::instrument(
    skip(pool),
    fields(post_id=%path.id)
)]
pub async fn hard_delete_post(
    path: web::Path<PostPathParams>,
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, PostError> {
    let post_id = path.id;

    let result = sqlx::query!(
        r#"
        DELETE FROM posts
	    WHERE id = $1
        "#,
        post_id
    )
    .execute(&**pool)
    .await
    .context("Failed to hard delete posts")?;

    if result.rows_affected() == 0 {
        return Err(PostError::NotFound);
    }

    Ok(HttpResponse::Ok().finish())
}
