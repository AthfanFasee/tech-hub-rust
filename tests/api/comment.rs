use crate::helpers::{TestApp, spawn_app};
use serde_json::json;
use sqlx::query;
use uuid::Uuid;

#[tokio::test]
async fn create_comment_returns_201_for_valid_input() {
    let app = spawn_app().await;
    app.login().await;

    let post_id = create_sample_post(&app).await;

    let payload = json!({
        "text": "This is a test comment",
        "post_id": post_id.to_string()
    });

    let response = app.create_comment(&payload).await;
    assert_eq!(
        response.status().as_u16(),
        201,
        "Expected 201 Created for valid comment creation"
    );

    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["post_id"], post_id.to_string());
    assert_eq!(body["text"], "This is a test comment");
}

#[tokio::test]
async fn create_comment_returns_400_for_invalid_post_id() {
    let app = spawn_app().await;
    app.login().await;

    let payload = json!({
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
    let app = spawn_app().await;
    app.login().await;

    let post_id = create_sample_post(&app).await;

    let payload = json!({
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
    let app = spawn_app().await;
    app.login().await;

    let post_id = create_sample_post(&app).await;
    app.logout().await;

    let payload = json!({
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

#[tokio::test]
async fn show_comments_for_post_returns_200_and_list() {
    let app = spawn_app().await;
    app.login().await;

    let post_id = create_sample_post(&app).await;

    for i in 0..3 {
        let payload = json!({
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

    let body: serde_json::Value = response.json().await.unwrap();
    let comments = body["comments"].as_array().unwrap();
    assert_eq!(comments.len(), 3);
    assert!(comments[0]["text"].is_string());
}

#[tokio::test]
async fn show_comments_returns_empty_array_for_post_with_no_comments() {
    let app = spawn_app().await;
    app.login().await;

    let post_id = create_sample_post(&app).await;

    app.logout().await;
    let response = app.get_comments(&post_id).await;

    assert_eq!(response.status().as_u16(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["comments"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn delete_comment_removes_comment_successfully() {
    let app = spawn_app().await;
    app.login().await;

    let post_id = create_sample_post(&app).await;

    let payload = json!({
        "text": "To be deleted",
        "post_id": post_id.to_string()
    });
    let resp = app.create_comment(&payload).await;
    assert_eq!(resp.status().as_u16(), 201);

    let body: serde_json::Value = resp.json().await.unwrap();
    let comment_id = Uuid::parse_str(body["id"].as_str().unwrap()).unwrap();

    let response = app.delete_comment(&comment_id).await;
    assert_eq!(
        response.status().as_u16(),
        200,
        "Expected 200 OK when deleting existing comment"
    );

    let record = query!(
        "SELECT COUNT(*) AS count FROM comments WHERE id = $1",
        comment_id
    )
    .fetch_one(&app.db_pool)
    .await
    .expect("Failed to check DB for deleted comment");

    assert_eq!(record.count.unwrap(), 0);
}

#[tokio::test]
async fn delete_comment_returns_404_for_nonexistent_comment() {
    let app = spawn_app().await;
    app.login().await;

    let random_id = Uuid::new_v4();
    let response = app.delete_comment(&random_id).await;

    assert_eq!(
        response.status().as_u16(),
        404,
        "Expected 404 for deleting non-existing comment"
    );
}

#[tokio::test]
async fn delete_comment_returns_401_if_unauthenticated() {
    let app = spawn_app().await;
    app.login().await;

    let post_id = create_sample_post(&app).await;
    let payload = json!({
        "text": "To test unauthorized delete",
        "post_id": post_id.to_string()
    });
    let resp = app.create_comment(&payload).await;
    assert_eq!(resp.status().as_u16(), 201);

    let body: serde_json::Value = resp.json().await.unwrap();
    let comment_id = Uuid::parse_str(body["id"].as_str().unwrap()).unwrap();

    app.logout().await;
    let response = app.delete_comment(&comment_id).await;

    assert_eq!(
        response.status().as_u16(),
        401,
        "Expected 401 for unauthenticated comment delete"
    );
}

async fn create_sample_post(app: &TestApp) -> Uuid {
    let payload = json!({
        "title": "Post for comments",
        "text": "This is a sample posts to attach comments to",
        "img": "https://example.com/posts.jpg"
    });

    let response = app.create_post(&payload).await;
    assert_eq!(response.status().as_u16(), 201);
    let body: serde_json::Value = response.json().await.unwrap();
    Uuid::parse_str(body["id"].as_str().unwrap()).unwrap()
}
