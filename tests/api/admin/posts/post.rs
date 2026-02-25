use crate::helpers;
use sqlx::query;
use uuid::Uuid;

// ============================================================================
// Hard Delete Post
// ============================================================================
#[tokio::test]
async fn hard_delete_post_removes_post_from_database() {
    let app = helpers::spawn_app().await;
    app.login_admin().await;

    let post_id = app.create_sample_post().await;

    let response = app.hard_delete_post(&post_id).await;
    assert_eq!(
        200,
        response.status().as_u16(),
        "Expected 200 OK for admin hard delete"
    );

    let result = query!("SELECT id FROM posts WHERE id = $1", post_id)
        .fetch_optional(&app.db_pool)
        .await
        .expect("Failed to query posts after hard delete");

    assert!(
        result.is_none(),
        "Expected post to be completely removed from DB after hard delete"
    );
}

#[tokio::test]
async fn hard_delete_post_returns_403_for_non_admins() {
    let app = helpers::spawn_app().await;
    app.login().await;

    let post_id = app.create_sample_post().await;

    let response = app.hard_delete_post(&post_id).await;
    assert_eq!(
        403,
        response.status().as_u16(),
        "Expected 403 Forbidden for non-admin attempting hard delete"
    );

    let result = query!("SELECT id FROM posts WHERE id = $1", post_id)
        .fetch_optional(&app.db_pool)
        .await
        .unwrap();

    assert!(result.is_some(), "Post should not be deleted by non-admin");
}

#[tokio::test]
async fn hard_delete_post_returns_404_for_nonexistent_post() {
    let app = helpers::spawn_app().await;
    app.login_admin().await;

    let random_id = Uuid::new_v4();
    let response = app.hard_delete_post(&random_id).await;

    assert_eq!(
        404,
        response.status().as_u16(),
        "Expected 404 when admin tries to delete non-existing post"
    );
}
