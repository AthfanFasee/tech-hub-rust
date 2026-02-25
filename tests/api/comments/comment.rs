use crate::helpers;
use serde_json::Value;
use sqlx::query;
use uuid::Uuid;

// ============================================================================
// Create Comment
// ============================================================================

#[tokio::test]
async fn create_comment_returns_201_for_valid_input() {
    let app = helpers::spawn_app().await;
    app.login().await;

    let post_id = app.create_sample_post().await;

    let payload = serde_json::json!({
        "text": "This is a test comment",
        "post_id": post_id.to_string()
    });

    let response = app.create_comment(&payload).await;
    assert_eq!(
        response.status().as_u16(),
        201,
        "Expected 201 Created for valid comment creation"
    );

    let body: Value = response.json().await.unwrap();
    assert_eq!(body["post_id"], post_id.to_string());
    assert_eq!(body["text"], "This is a test comment");
}

#[tokio::test]
async fn create_comment_returns_400_for_invalid_post_id() {
    let app = helpers::spawn_app().await;
    app.login().await;

    let payload = serde_json::json!({
        "text": "Invalid posts id test",
        "post_id": "not-a-uuid"
    });

    let response = app.create_comment(&payload).await;

    assert_eq!(
        response.status().as_u16(),
        400,
        "Expected 400 for invalid UUID post_id"
    );
}

#[tokio::test]
async fn create_comment_returns_400_for_empty_text() {
    let app = helpers::spawn_app().await;
    app.login().await;

    let post_id = app.create_sample_post().await;

    let payload = serde_json::json!({
        "text": "",
        "post_id": post_id.to_string()
    });

    let response = app.create_comment(&payload).await;
    assert_eq!(
        response.status().as_u16(),
        400,
        "Expected 400 for empty comment text"
    );
}

#[tokio::test]
async fn create_comment_returns_401_if_unauthenticated() {
    let app = helpers::spawn_app().await;
    app.login().await;

    let post_id = app.create_sample_post().await;
    app.logout().await;

    let payload = serde_json::json!({
        "text": "Comment without login",
        "post_id": post_id.to_string()
    });

    let response = app.create_comment(&payload).await;

    assert_eq!(
        response.status().as_u16(),
        401,
        "Expected 401 for unauthenticated comment creation"
    );
}

// ============================================================================
// Get Comments
// ============================================================================

#[tokio::test]
async fn get_comments_returns_200_with_comment_list() {
    let app = helpers::spawn_app().await;
    app.login().await;

    let post_id = app.create_sample_post().await;

    for i in 0..3 {
        let payload = serde_json::json!({
            "text": format!("Comment {}", i),
            "post_id": post_id.to_string()
        });
        let resp = app.create_comment(&payload).await;
        assert_eq!(resp.status().as_u16(), 201);
    }

    app.logout().await;

    let response = app.get_comments(&post_id).await;
    assert_eq!(
        response.status().as_u16(),
        200,
        "Expected 200 OK when fetching comments for existing post"
    );

    let body: Value = response.json().await.unwrap();
    let comments = body["comments"].as_array().unwrap();
    assert_eq!(comments.len(), 3);
    assert!(comments[0]["text"].is_string());
}

#[tokio::test]
async fn get_comments_returns_empty_array_for_post_with_no_comments() {
    let app = helpers::spawn_app().await;
    app.login().await;

    let post_id = app.create_sample_post().await;

    app.logout().await;
    let response = app.get_comments(&post_id).await;

    assert_eq!(response.status().as_u16(), 200);
    let body: Value = response.json().await.unwrap();
    assert!(body["comments"].as_array().unwrap().is_empty());
}

// ============================================================================
// Delete Comment
// ============================================================================

#[tokio::test]
async fn delete_comment_returns_401_if_unauthenticated() {
    let app = helpers::spawn_app().await;
    app.login().await;

    let post_id = app.create_sample_post().await;
    let payload = serde_json::json!({
        "text": "To test unauthorized delete",
        "post_id": post_id.to_string()
    });
    let resp = app.create_comment(&payload).await;
    assert_eq!(resp.status().as_u16(), 201);

    let body: Value = resp.json().await.unwrap();
    let comment_id = Uuid::parse_str(body["id"].as_str().unwrap()).unwrap();

    app.logout().await;
    let response = app.delete_comment(&comment_id).await;

    assert_eq!(
        401,
        response.status().as_u16(),
        "Expected 401 for unauthenticated comment delete"
    );
}

#[tokio::test]
async fn delete_comment_only_creator_or_admin_can_delete_comment() {
    let app = helpers::spawn_app().await;
    app.login().await;

    let post_id = app.create_sample_post().await;

    // User A creates comment
    let payload = serde_json::json!({
        "text": "Comment A",
        "post_id": post_id.to_string()
    });
    let resp = app.create_comment(&payload).await;
    let body: Value = resp.json().await.unwrap();
    let comment_id = Uuid::parse_str(body["id"].as_str().unwrap()).unwrap();
    app.logout().await;

    // User B logs in and tries to delete (should fail with 403)
    let user_b = app.create_activated_user().await;
    app.login_with(&user_b).await;

    let response = app.delete_comment(&comment_id).await;
    assert_eq!(
        403,
        response.status().as_u16(),
        "Expected 403 Forbidden when non-creator tries to delete comment"
    );

    // Admin logs in and deletes (should succeed)
    app.login_admin().await;

    let response = app.delete_comment(&comment_id).await;
    assert_eq!(
        200,
        response.status().as_u16(),
        "Expected 200 OK when admin deletes comment"
    );

    let record = query!(
        "SELECT COUNT(*) AS count FROM comments WHERE id = $1",
        comment_id
    )
    .fetch_one(&app.db_pool)
    .await
    .expect("Failed to check DB");

    assert_eq!(record.count.unwrap(), 0, "Comment should be deleted");
}

#[tokio::test]
async fn delete_comment_removes_comment_successfully() {
    let app = helpers::spawn_app().await;
    app.login().await;

    let post_id = app.create_sample_post().await;

    let payload = serde_json::json!({
        "text": "To be deleted",
        "post_id": post_id.to_string()
    });
    let resp = app.create_comment(&payload).await;
    assert_eq!(resp.status().as_u16(), 201);

    let body: Value = resp.json().await.unwrap();
    let comment_id = Uuid::parse_str(body["id"].as_str().unwrap()).unwrap();

    let response = app.delete_comment(&comment_id).await;
    assert_eq!(response.status().as_u16(), 200);

    let record = query!(
        "SELECT COUNT(*) AS count FROM comments WHERE id = $1",
        comment_id
    )
    .fetch_one(&app.db_pool)
    .await
    .expect("Failed to check DB");

    assert_eq!(record.count.unwrap(), 0);
}

#[tokio::test]
async fn delete_comment_returns_404_for_nonexistent_comment_when_authorized() {
    let app = helpers::spawn_app().await;
    app.login_admin().await;

    let random_id = Uuid::new_v4();
    let response = app.delete_comment(&random_id).await;

    assert_eq!(
        response.status().as_u16(),
        404,
        "Expected 404 when an authorized user attempts to delete a non-existing comment"
    );
}

#[tokio::test]
async fn delete_comment_does_not_leak_existence_information() {
    let app = helpers::spawn_app().await;
    let random_comment = Uuid::new_v4();

    let user_b = app.create_activated_user().await;
    app.login_with(&user_b).await;

    let response = app.delete_comment(&random_comment).await;
    assert_eq!(
        403,
        response.status().as_u16(),
        "Expected 403 forbidden (not 404) for unauthorized delete attempt"
    );
}
