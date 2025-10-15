use crate::helpers::{TestApp, spawn_app};
use serde_json::json;
use sqlx::query;
use uuid::Uuid;

#[tokio::test]
async fn user_must_be_logged_in_to_create_post() {
    let app = spawn_app().await;

    let payload = json!({
        "title": "Some title",
        "text": "Post content here...",
        "img": "https://example.com/image.jpg"
    });

    let response = app.create_post(&payload).await;

    assert_eq!(
        401,
        response.status().as_u16(),
        "The API did not return 401 Unauthorized for unauthenticated user."
    );
}

#[tokio::test]
async fn create_post_returns_400_for_invalid_payload() {
    let app = spawn_app().await;
    app.login().await;

    let invalid_payloads = vec![
        json!({ "title": "", "text": "Some text", "img": "https://example.com/image.jpg" }),
        json!({ "title": "Title", "text": "", "img": "https://example.com/image.jpg" }),
        json!({ "title": "Title", "text": "Text", "img": "" }),
        json!({}),
    ];

    for payload in invalid_payloads {
        let response = app.create_post(&payload).await;
        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not return 400 for invalid input: {payload:?}"
        );
    }
}

#[tokio::test]
async fn create_post_persists_valid_post_and_returns_201() {
    let app = spawn_app().await;
    app.login().await;

    let payload = json!({
        "title": "My first blog post",
        "text": "This is a test post",
        "img": "https://example.com/img.jpg"
    });

    let response = app.create_post(&payload).await;
    assert_eq!(response.status().as_u16(), 201);

    let body: serde_json::Value = response.json().await.unwrap();

    assert_eq!(body["title"], "My first blog post");
    assert_eq!(body["post_text"], "This is a test post");
    assert_eq!(body["img"], "https://example.com/img.jpg");
    assert!(body.get("id").is_some(), "Missing 'id' field in response");
    assert!(
        body.get("created_at").is_some(),
        "Missing 'created_at' field in response"
    );

    assert_eq!(
        body["created_by"].as_str().unwrap(),
        app.test_user.user_id.to_string(),
        "The post's created_by field does not match the logged-in user"
    );

    let saved = sqlx::query!(
        r#"
        SELECT id, title, post_text, img, created_at, created_by
        FROM posts
        WHERE title = $1
        "#,
        "My first blog post"
    )
    .fetch_one(&app.db_pool)
    .await
    .expect("Failed to fetch saved post.");

    assert_eq!(saved.title, "My first blog post");
    assert_eq!(saved.post_text, "This is a test post");
    assert_eq!(saved.img, "https://example.com/img.jpg");

    assert_eq!(
        saved.created_by, app.test_user.user_id,
        "Post was not attributed to the logged-in user"
    );
}

#[tokio::test]
async fn user_must_be_logged_in_to_update_post() {
    let app = spawn_app().await;
    let post_id = Uuid::new_v4();

    let payload = json!({
        "title": "Updated title",
        "text": "Updated content",
        "img": "https://example.com/updated.jpg"
    });

    let response = app.update_post(&post_id, &payload).await;

    assert_eq!(
        401,
        response.status().as_u16(),
        "The API did not return 401 Unauthorized for unauthenticated user."
    );
}

#[tokio::test]
async fn update_post_returns_400_for_invalid_payload() {
    let app = spawn_app().await;
    app.login().await;

    let post_id = create_sample_post(&app).await;

    let invalid_payloads = vec![
        json!({ "title": "", "text": "Some text", "img": "https://example.com/img.jpg" }),
        json!({ "title": "Title", "text": "", "img": "https://example.com/img.jpg" }),
        json!({ "title": "Title", "text": "Text", "img": "" }),
        json!({}),
    ];

    for payload in invalid_payloads {
        let response = app.update_post(&post_id, &payload).await;
        assert_eq!(
            400,
            response.status().as_u16(),
            "Expected 400 for invalid payload, got {} instead",
            response.status()
        );
    }
}

#[tokio::test]
async fn update_post_returns_404_if_not_found() {
    let app = spawn_app().await;
    app.login().await;

    let payload = json!({
        "title": "Updated title",
        "text": "Updated text",
        "img": "https://example.com/updated.jpg"
    });

    let response = app.update_post(&Uuid::new_v4(), &payload).await;

    assert_eq!(
        404,
        response.status().as_u16(),
        "Expected 404 Not Found for non-existing post."
    );
}

#[tokio::test]
async fn update_post_persists_changes_and_returns_200() {
    let app = spawn_app().await;
    app.login().await;

    let post_id = create_sample_post(&app).await;

    let payload = json!({
        "title": "Updated Title",
        "text": "Updated post content",
        "img": "https://example.com/updated.jpg"
    });

    let response = app.update_post(&post_id, &payload).await;
    assert_eq!(response.status().as_u16(), 200, "Update failed");

    let body: serde_json::Value = response.json().await.unwrap();
    let post = &body["post"];

    assert_eq!(post["title"], "Updated Title");
    assert_eq!(post["text"], "Updated post content");
    assert_eq!(post["img"], "https://example.com/updated.jpg");

    let record = query!(
        r#"
        SELECT title, post_text, img, version
        FROM posts
        WHERE id = $1
        "#,
        post_id
    )
    .fetch_one(&app.db_pool)
    .await
    .expect("Failed to fetch updated post");

    assert_eq!(record.title, "Updated Title");
    assert_eq!(record.post_text, "Updated post content");
    assert_eq!(record.img, "https://example.com/updated.jpg");
    assert!(record.version > 1, "Version should have been incremented");
}

#[tokio::test]
async fn delete_post_marks_post_as_deleted() {
    let app = spawn_app().await;
    app.login().await;

    let post_id = create_sample_post(&app).await;

    let response = app.delete_post(&post_id).await;
    assert_eq!(
        200,
        response.status().as_u16(),
        "Expected 200 OK on successful soft delete"
    );

    let record = query!("SELECT deleted_at FROM posts WHERE id = $1", post_id)
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch deleted post");

    assert!(
        record.deleted_at.is_some(),
        "Expected deleted_at to be set after soft deletion"
    );
}

#[tokio::test]
async fn delete_post_returns_404_for_nonexistent_id() {
    let app = spawn_app().await;
    app.login().await;

    let random_id = Uuid::new_v4();
    let response = app.delete_post(&random_id).await;

    assert_eq!(
        404,
        response.status().as_u16(),
        "Expected 404 for non-existing post id"
    );
}

#[tokio::test]
async fn delete_post_returns_404_if_already_deleted() {
    let app = spawn_app().await;
    app.login().await;

    let post_id = create_sample_post(&app).await;

    app.delete_post(&post_id).await;

    let response = app.delete_post(&post_id).await;
    assert_eq!(
        404,
        response.status().as_u16(),
        "Expected 404 on deleting an already deleted post"
    );
}

#[tokio::test]
async fn delete_post_requires_authentication() {
    let app = spawn_app().await;

    let random_id = Uuid::new_v4();
    let response = app.delete_post(&random_id).await;

    assert_eq!(
        401,
        response.status().as_u16(),
        "Expected 401 when unauthenticated user tries to delete post"
    );
}

#[tokio::test]
async fn hard_delete_post_removes_from_database() {
    let app = spawn_app().await;
    app.login_admin().await;

    let post_id = create_sample_post(&app).await;

    let response = app.hard_delete_post(&post_id).await;
    assert_eq!(
        200,
        response.status().as_u16(),
        "Expected 200 OK for admin hard delete"
    );

    let result = query!("SELECT id FROM posts WHERE id = $1", post_id)
        .fetch_optional(&app.db_pool)
        .await
        .expect("Failed to query post after hard delete");

    assert!(
        result.is_none(),
        "Expected post to be completely removed from DB after hard delete"
    );
}

#[tokio::test]
async fn hard_delete_requires_admin_privileges() {
    let app = spawn_app().await;
    app.login().await;

    let post_id = create_sample_post(&app).await;

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
async fn hard_delete_returns_404_for_nonexistent_post() {
    let app = spawn_app().await;
    app.login_admin().await;

    let random_id = Uuid::new_v4();
    let response = app.hard_delete_post(&random_id).await;

    assert_eq!(
        404,
        response.status().as_u16(),
        "Expected 404 when admin tries to delete non-existing post"
    );
}

#[tokio::test]
async fn like_post_adds_user_to_liked_by() {
    let app = spawn_app().await;
    app.login().await;

    let post_id = create_sample_post(&app).await;
    let user_id = app.test_user.user_id;

    let response = app.like_post(&post_id).await;
    assert_eq!(response.status().as_u16(), 200, "Like request failed");

    let record = query!(
        r#"
        SELECT liked_by
        FROM posts
        WHERE id = $1
        "#,
        post_id
    )
    .fetch_one(&app.db_pool)
    .await
    .expect("Failed to fetch post after like");

    assert!(
        record.liked_by.contains(&user_id),
        "Expected liked_by to contain user_id after liking post"
    );
}

#[tokio::test]
async fn like_post_is_idempotent_for_same_user() {
    let app = spawn_app().await;
    app.login().await;

    let post_id = create_sample_post(&app).await;
    let user_id = app.test_user.user_id;

    // Like twice
    app.like_post(&post_id).await;
    app.like_post(&post_id).await;

    let record = query!(
        r#"
        SELECT liked_by
        FROM posts
        WHERE id = $1
        "#,
        post_id
    )
    .fetch_one(&app.db_pool)
    .await
    .expect("Failed to fetch post after like");

    let count = record.liked_by.iter().filter(|&&id| id == user_id).count();

    assert_eq!(count, 1, "Expected exactly one like from this user");
}

#[tokio::test]
async fn like_post_returns_404_for_nonexistent_post() {
    let app = spawn_app().await;
    app.login().await;

    let random_id = Uuid::new_v4();
    let response = app.like_post(&random_id).await;

    assert_eq!(
        response.status().as_u16(),
        404,
        "Expected 404 for liking non-existing post"
    );
}

#[tokio::test]
async fn like_post_returns_401_if_unauthenticated() {
    let app = spawn_app().await;
    app.login().await;

    let post_id = create_sample_post(&app).await;
    app.logout().await;

    let response = app.like_post(&post_id).await;
    assert_eq!(
        response.status().as_u16(),
        401,
        "Expected 401 for unauthenticated like request"
    );
}

#[tokio::test]
async fn dislike_post_removes_user_from_liked_by() {
    let app = spawn_app().await;
    app.login().await;

    let post_id = create_sample_post(&app).await;
    let user_id = app.test_user.user_id;

    app.like_post(&post_id).await;

    let response = app.dislike_post(&post_id).await;
    assert_eq!(response.status().as_u16(), 200, "Dislike request failed");

    let record = query!(
        r#"
        SELECT liked_by
        FROM posts
        WHERE id = $1
        "#,
        post_id
    )
    .fetch_one(&app.db_pool)
    .await
    .expect("Failed to fetch post after dislike");

    assert!(
        !record.liked_by.contains(&user_id),
        "Expected liked_by to not contain user_id after dislike"
    );
}

#[tokio::test]
async fn dislike_post_returns_404_for_nonexistent_post() {
    let app = spawn_app().await;
    app.login().await;

    let random_id = Uuid::new_v4();
    let response = app.dislike_post(&random_id).await;

    assert_eq!(
        response.status().as_u16(),
        404,
        "Expected 404 for disliking non-existing post"
    );
}

#[tokio::test]
async fn dislike_post_returns_401_if_unauthenticated() {
    let app = spawn_app().await;
    app.login().await;

    let post_id = create_sample_post(&app).await;
    app.logout().await;

    let response = app.dislike_post(&post_id).await;
    assert_eq!(
        response.status().as_u16(),
        401,
        "Expected 401 for unauthenticated dislike request"
    );
}

#[tokio::test]
async fn get_post_returns_post_data_successfully() {
    let app = spawn_app().await;
    app.login().await;

    let post_id = create_sample_post(&app).await;
    let response = app.get_post(&post_id).await;
    assert_eq!(
        response.status().as_u16(),
        200,
        "Expected 200 OK when fetching an existing post"
    );

    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["post"]["id"], post_id.to_string());
    assert!(body["post"]["title"].is_string());
}

#[tokio::test]
async fn get_post_returns_404_for_nonexistent_post() {
    let app = spawn_app().await;
    app.login().await;

    let random_id = Uuid::new_v4();
    let response = app.get_post(&random_id).await;
    assert_eq!(
        response.status().as_u16(),
        404,
        "Expected 404 for non-existing post"
    );
}

#[tokio::test]
async fn get_post_does_return_200_if_unauthenticated() {
    let app = spawn_app().await;
    app.login().await;

    let post_id = create_sample_post(&app).await;
    app.logout().await;

    let response = app.get_post(&post_id).await;
    assert_eq!(
        response.status().as_u16(),
        200,
        "Expected 200 OK when fetching an existing post as unauthenticated user"
    );
}

#[tokio::test]
async fn get_post_does_not_return_deleted_posts() {
    let app = spawn_app().await;
    app.login().await;

    let post_id = create_sample_post(&app).await;

    // Soft delete manually
    query!(
        r#"
        UPDATE posts
        SET deleted_at = now()
        WHERE id = $1
        "#,
        post_id
    )
    .execute(&app.db_pool)
    .await
    .expect("Failed to soft delete post");

    let response = app.get_post(&post_id).await;

    assert_eq!(
        response.status().as_u16(),
        404,
        "Expected 404 for soft-deleted post"
    );
}

async fn create_sample_post(app: &TestApp) -> Uuid {
    let payload = json!({
        "title": "Initial title",
        "text": "Initial text",
        "img": "https://example.com/initial.jpg"
    });

    let response = app.create_post(&payload).await;
    assert_eq!(
        response.status().as_u16(),
        201,
        "Failed to create sample post"
    );
    let body: serde_json::Value = response.json().await.unwrap();
    Uuid::parse_str(body["id"].as_str().unwrap()).unwrap()
}
