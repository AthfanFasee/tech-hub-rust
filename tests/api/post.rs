use crate::helpers;
use sqlx::query;
use uuid::Uuid;

// ============================================================================
// Create Post
// ============================================================================

#[tokio::test]
async fn user_must_be_logged_in_to_create_post() {
    let app = helpers::spawn_app().await;

    let payload = serde_json::json!({
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
    let app = helpers::spawn_app().await;
    app.login().await;

    let invalid_payloads = vec![
        serde_json::json!({ "title": "", "text": "Some text", "img": "https://example.com/image.jpg" }),
        serde_json::json!({ "title": "Title", "text": "", "img": "https://example.com/image.jpg" }),
        serde_json::json!({ "title": "Title", "text": "Text", "img": "" }),
        serde_json::json!({}),
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
    let app = helpers::spawn_app().await;
    app.login().await;

    let payload = serde_json::json!({
        "title": "My first blog posts",
        "text": "This is a test posts",
        "img": "https://example.com/img.jpg"
    });

    let response = app.create_post(&payload).await;
    assert_eq!(response.status().as_u16(), 201);

    let body: serde_json::Value = response.json().await.unwrap();

    assert_eq!(body["title"], "My first blog posts");
    assert_eq!(body["post_text"], "This is a test posts");
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
        "My first blog posts"
    )
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved posts.");

    assert_eq!(saved.title, "My first blog posts");
    assert_eq!(saved.post_text, "This is a test posts");
    assert_eq!(saved.img, "https://example.com/img.jpg");

    assert_eq!(
        saved.created_by, app.test_user.user_id,
        "Post was not attributed to the logged-in user"
    );
}

// ============================================================================
// Update Post
// ============================================================================

#[tokio::test]
async fn user_must_be_logged_in_to_update_post() {
    let app = helpers::spawn_app().await;
    let post_id = Uuid::new_v4();

    let payload = serde_json::json!({
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
async fn non_creator_non_admin_cannot_update_post() {
    let app = helpers::spawn_app().await;
    app.login().await;

    let post_id = app.create_sample_post().await;

    // logout, create a different user
    app.logout().await;
    let payload_user = app.create_activated_user().await;
    app.login_with(&payload_user).await;

    let payload = serde_json::json!({
        "title": "Hacked title",
        "text": "Hacked text",
        "img": "https://example.com/hacked.jpg"
    });

    let response = app.update_post(&post_id, &payload).await;

    assert_eq!(
        403,
        response.status().as_u16(),
        "Expected 403 Forbidden when non-creator tries to update post"
    );
}

#[tokio::test]
async fn admin_can_update_post_created_by_someone_else() {
    let app = helpers::spawn_app().await;
    app.login().await;

    let post_id = app.create_sample_post().await;
    app.logout().await;

    app.login_admin().await;

    let payload = serde_json::json!({
        "title": "Admin Updated",
        "text": "Admin text",
        "img": "https://example.com/admin.jpg"
    });

    let response = app.update_post(&post_id, &payload).await;
    assert_eq!(
        200,
        response.status().as_u16(),
        "Admin should be able to update any post"
    );
}

#[tokio::test]
async fn update_post_returns_403_for_nonexistent_id_when_unauthorized() {
    let app = helpers::spawn_app().await;

    app.login().await; // normal user

    let payload = serde_json::json!({
        "title": "Updated title",
        "text": "Updated text",
        "img": "https://example.com/updated.jpg"
    });

    let response = app.update_post(&Uuid::new_v4(), &payload).await;

    assert_eq!(
        403,
        response.status().as_u16(),
        "Unauthorized users should not learn whether a post exists"
    );
}

#[tokio::test]
async fn update_post_returns_404_for_nonexistent_id_when_authorized() {
    let app = helpers::spawn_app().await;
    app.login_admin().await;

    let payload = serde_json::json!({
        "title": "Updated title",
        "text": "Updated text",
        "img": "https://example.com/updated.jpg"
    });

    let response = app.update_post(&Uuid::new_v4(), &payload).await;

    assert_eq!(
        404,
        response.status().as_u16(),
        "Authorized requests should return 404 for missing post id"
    );
}

#[tokio::test]
async fn update_post_returns_400_for_invalid_payload() {
    let app = helpers::spawn_app().await;
    app.login().await;

    let post_id = app.create_sample_post().await;

    let invalid_payloads = vec![
        serde_json::json!({ "title": "", "text": "Some text", "img": "https://example.com/img.jpg" }),
        serde_json::json!({ "title": "Title", "text": "", "img": "https://example.com/img.jpg" }),
        serde_json::json!({ "title": "Title", "text": "Text", "img": "" }),
        serde_json::json!({}),
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
async fn update_post_persists_changes_and_returns_200() {
    let app = helpers::spawn_app().await;
    app.login().await;

    let post_id = app.create_sample_post().await;

    let payload = serde_json::json!({
        "title": "Updated Title",
        "text": "Updated posts content",
        "img": "https://example.com/updated.jpg"
    });

    let response = app.update_post(&post_id, &payload).await;
    assert_eq!(response.status().as_u16(), 200, "Update failed");

    let body: serde_json::Value = response.json().await.unwrap();
    let post = &body["posts"];

    assert_eq!(post["title"], "Updated Title");
    assert_eq!(post["text"], "Updated posts content");
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
        .expect("Failed to fetch updated posts");

    assert_eq!(record.title, "Updated Title");
    assert_eq!(record.post_text, "Updated posts content");
    assert_eq!(record.img, "https://example.com/updated.jpg");
    assert!(record.version > 1, "Version should have been incremented");
}

// ============================================================================
// Delete Post
// ============================================================================

#[tokio::test]
async fn delete_post_marks_post_as_deleted() {
    let app = helpers::spawn_app().await;
    app.login().await;

    let post_id = app.create_sample_post().await;

    let response = app.delete_post(&post_id).await;
    assert_eq!(
        200,
        response.status().as_u16(),
        "Expected 200 OK on successful soft delete"
    );

    let record = query!("SELECT deleted_at FROM posts WHERE id = $1", post_id)
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch deleted posts");

    assert!(
        record.deleted_at.is_some(),
        "Expected deleted_at to be set after soft deletion"
    );
}

#[tokio::test]
async fn post_can_only_be_deleted_by_creator_or_an_admin() {
    let app = helpers::spawn_app().await;
    app.login().await;

    let post_id = app.create_sample_post().await;

    // logout, create a different user
    app.logout().await;

    let payload = app.create_activated_user().await;
    app.login_with(&payload).await;

    let response = app.delete_post(&post_id).await;
    assert_eq!(
        403,
        response.status().as_u16(),
        "Expected 403 forbidden request when non creator tries to delete a post"
    );

    app.login_admin().await;

    let response = app.delete_post(&post_id).await;
    assert_eq!(
        200,
        response.status().as_u16(),
        "Expected 200 OK when an admin deletes a post created by someone else"
    );

    let record = query!("SELECT deleted_at FROM posts WHERE id = $1", post_id)
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch deleted posts");

    assert!(
        record.deleted_at.is_some(),
        "Expected deleted_at to be set after when an admin deletes a post created by someone else"
    );
}

#[tokio::test]
async fn forbidden_delete_post_request_does_not_leak_ownership_information() {
    let app = helpers::spawn_app().await;
    app.login().await;

    let post_id = app.create_sample_post().await;
    app.logout().await;

    let user_b = app.create_activated_user().await;
    app.login_with(&user_b).await;

    let response = app.delete_post(&post_id).await;
    let status = response.status().as_u16();
    let body = response.text().await.unwrap();

    assert_eq!(
        403, status,
        "Expected 403 when a non-owner tries to delete a post"
    );

    // The error message must be generic and not leak ownership details
    assert!(
        body.contains("not authorized to perform this action"),
        "Expected generic forbidden message. Actual body: {}",
        body
    );

    // The post still exists (not soft deleted)
    let record = query!("SELECT deleted_at FROM posts WHERE id = $1", post_id)
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch deleted_at column");

    assert!(
        record.deleted_at.is_none(),
        "Post should not be deleted when a non-owner attempts deletion"
    );
}

#[tokio::test]
async fn delete_post_returns_404_for_nonexistent_id_for_admin() {
    let app = helpers::spawn_app().await;
    app.login_admin().await;

    let random_id = Uuid::new_v4();
    let response = app.delete_post(&random_id).await;

    assert_eq!(
        404,
        response.status().as_u16(),
        "Expected 404 for non-existing post id"
    );
}

#[tokio::test]
async fn delete_post_returns_403_for_nonexistent_id_for_non_admin_user() {
    let app = helpers::spawn_app().await;
    app.login().await;

    let random_id = Uuid::new_v4();
    let response = app.delete_post(&random_id).await;

    assert_eq!(
        403,
        response.status().as_u16(),
        "Expected 403 for delete attempt by unauthorized user"
    );
}

#[tokio::test]
async fn delete_post_returns_404_if_already_deleted() {
    let app = helpers::spawn_app().await;
    app.login_admin().await;

    let post_id = app.create_sample_post().await;

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
    let app = helpers::spawn_app().await;

    let random_id = Uuid::new_v4();
    let response = app.delete_post(&random_id).await;

    assert_eq!(
        401,
        response.status().as_u16(),
        "Expected 401 when unauthenticated user tries to delete post"
    );
}

// ============================================================================
// Hard Delete Post
// ============================================================================

#[tokio::test]
async fn hard_delete_post_removes_from_database() {
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
async fn hard_delete_requires_admin_privileges() {
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
async fn hard_delete_returns_404_for_nonexistent_post() {
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

// ============================================================================
// Like Post
// ============================================================================

#[tokio::test]
async fn like_post_adds_user_to_liked_by() {
    let app = helpers::spawn_app().await;
    app.login().await;

    let post_id = app.create_sample_post().await;
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
        .expect("Failed to fetch posts after like");

    assert!(
        record.liked_by.contains(&user_id),
        "Expected liked_by to contain user_id after liking post"
    );
}

#[tokio::test]
async fn like_post_is_idempotent_for_same_user() {
    let app = helpers::spawn_app().await;
    app.login().await;

    let post_id = app.create_sample_post().await;
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
        .expect("Failed to fetch posts after like");

    let count = record.liked_by.iter().filter(|&&id| id == user_id).count();

    assert_eq!(count, 1, "Expected exactly one like from this user");
}

#[tokio::test]
async fn like_post_returns_404_for_nonexistent_post() {
    let app = helpers::spawn_app().await;
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
    let app = helpers::spawn_app().await;
    app.login().await;

    let post_id = app.create_sample_post().await;
    app.logout().await;

    let response = app.like_post(&post_id).await;
    assert_eq!(
        response.status().as_u16(),
        401,
        "Expected 401 for unauthenticated like request"
    );
}

// ============================================================================
// Dislike Post
// ============================================================================

#[tokio::test]
async fn dislike_post_removes_user_from_liked_by() {
    let app = helpers::spawn_app().await;
    app.login().await;

    let post_id = app.create_sample_post().await;
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
        .expect("Failed to fetch posts after dislike");

    assert!(
        !record.liked_by.contains(&user_id),
        "Expected liked_by to not contain user_id after dislike"
    );
}

#[tokio::test]
async fn dislike_post_returns_404_for_nonexistent_post() {
    let app = helpers::spawn_app().await;
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
    let app = helpers::spawn_app().await;
    app.login().await;

    let post_id = app.create_sample_post().await;
    app.logout().await;

    let response = app.dislike_post(&post_id).await;
    assert_eq!(
        response.status().as_u16(),
        401,
        "Expected 401 for unauthenticated dislike request"
    );
}

// ============================================================================
// Get Post
// ============================================================================

#[tokio::test]
async fn get_post_returns_post_data_successfully() {
    let app = helpers::spawn_app().await;
    app.login().await;

    let post_id = app.create_sample_post().await;
    let response = app.get_post(&post_id).await;
    assert_eq!(
        response.status().as_u16(),
        200,
        "Expected 200 OK when fetching an existing post"
    );

    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["posts"]["id"], post_id.to_string());
    assert!(body["posts"]["title"].is_string());
}

#[tokio::test]
async fn get_post_returns_404_for_nonexistent_post() {
    let app = helpers::spawn_app().await;
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
    let app = helpers::spawn_app().await;
    app.login().await;

    let post_id = app.create_sample_post().await;
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
    let app = helpers::spawn_app().await;
    app.login().await;

    let post_id = app.create_sample_post().await;

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
        .expect("Failed to soft delete posts");

    let response = app.get_post(&post_id).await;

    assert_eq!(
        response.status().as_u16(),
        404,
        "Expected 404 for soft-deleted post"
    );
}
