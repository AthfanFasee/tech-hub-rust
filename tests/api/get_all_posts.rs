use crate::helpers::{TestApp, spawn_app};
use serde_json::json;
use uuid::Uuid;

async fn create_sample_post(app: &TestApp, title: &str, text: &str) -> Uuid {
    let payload = json!({
        "title": title,
        "text": text,
        "img": "https://example.com/sample.jpg"
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

async fn like_post_as_user(app: &TestApp, post_id: &Uuid) {
    let response = app.like_post(post_id).await;
    assert_eq!(response.status().as_u16(), 200, "Failed to like post");
}

// ============================================================================
// Basic Functionality Tests
// ============================================================================

#[tokio::test]
async fn get_all_posts_returns_posts_successfully() {
    let app = spawn_app().await;
    app.login().await;

    create_sample_post(&app, "First Post", "First content").await;
    create_sample_post(&app, "Second Post", "Second content").await;

    let response = app.get_all_posts("").await;
    assert_eq!(
        response.status().as_u16(),
        200,
        "Expected 200 OK when fetching posts"
    );

    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["posts"].is_array());
    assert!(body["metadata"].is_object());
    assert_eq!(body["posts"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn get_all_posts_works_without_authentication() {
    let app = spawn_app().await;
    app.login().await;

    create_sample_post(&app, "Public Post", "Public content").await;

    app.logout().await;

    let response = app.get_all_posts("").await;
    assert_eq!(
        response.status().as_u16(),
        200,
        "Expected 200 OK when fetching posts without authentication"
    );

    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["posts"].is_array());
}

#[tokio::test]
async fn get_all_posts_returns_empty_array_when_no_posts() {
    let app = spawn_app().await;

    let response = app.get_all_posts("").await;
    assert_eq!(
        response.status().as_u16(),
        200,
        "Expected 200 OK even when no posts exist"
    );

    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["posts"].as_array().unwrap().len(), 0);
    assert_eq!(body["metadata"]["total_records"], 0);
}

// ============================================================================
// Pagination Tests
// ============================================================================

#[tokio::test]
async fn get_all_posts_respects_pagination_limit() {
    let app = spawn_app().await;
    app.login().await;

    for i in 1..=5 {
        create_sample_post(&app, &format!("Post {i}"), "Content").await;
    }

    let response = app.get_all_posts("?limit=2").await;
    assert_eq!(response.status().as_u16(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(
        body["posts"].as_array().unwrap().len(),
        2,
        "Expected only 2 posts due to limit"
    );
    assert_eq!(body["metadata"]["page_size"], 2);
    assert_eq!(body["metadata"]["total_records"], 5);
}

#[tokio::test]
async fn get_all_posts_respects_page_parameter() {
    let app = spawn_app().await;
    app.login().await;

    for i in 1..=5 {
        create_sample_post(&app, &format!("Post {i}"), "Content").await;
    }

    let response = app.get_all_posts("?limit=2&page=2").await;
    assert_eq!(response.status().as_u16(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["metadata"]["current_page"], 2);
    assert_eq!(body["posts"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn get_all_posts_returns_correct_metadata() {
    let app = spawn_app().await;
    app.login().await;

    for i in 1..=10 {
        create_sample_post(&app, &format!("Post {i}"), "Content").await;
    }

    let response = app.get_all_posts("?limit=3&page=1").await;
    assert_eq!(response.status().as_u16(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["metadata"]["current_page"], 1);
    assert_eq!(body["metadata"]["page_size"], 3);
    assert_eq!(body["metadata"]["first_page"], 1);
    assert_eq!(body["metadata"]["last_page"], 4); // 10 posts / 3 per page = 4 pages
    assert_eq!(body["metadata"]["total_records"], 10);
}

// ============================================================================
// Validation Tests
// ============================================================================

#[tokio::test]
async fn get_all_posts_rejects_invalid_page_zero() {
    let app = spawn_app().await;

    let response = app.get_all_posts("?page=0").await;
    assert_eq!(response.status().as_u16(), 400, "Expected 400 for page=0");
}

#[tokio::test]
async fn get_all_posts_rejects_invalid_page_negative() {
    let app = spawn_app().await;

    let response = app.get_all_posts("?page=-1").await;
    assert_eq!(
        response.status().as_u16(),
        400,
        "Expected 400 for negative page"
    );
}

#[tokio::test]
async fn get_all_posts_rejects_page_exceeding_maximum() {
    let app = spawn_app().await;

    let response = app.get_all_posts("?page=1000001").await;
    assert_eq!(
        response.status().as_u16(),
        400,
        "Expected 400 for page > 1,000,000"
    );
}

#[tokio::test]
async fn get_all_posts_rejects_invalid_limit_zero() {
    let app = spawn_app().await;

    let response = app.get_all_posts("?limit=0").await;
    assert_eq!(response.status().as_u16(), 400, "Expected 400 for limit=0");
}

#[tokio::test]
async fn get_all_posts_rejects_limit_exceeding_maximum() {
    let app = spawn_app().await;

    let response = app.get_all_posts("?limit=101").await;
    assert_eq!(
        response.status().as_u16(),
        400,
        "Expected 400 for limit > 100"
    );
}

#[tokio::test]
async fn get_all_posts_rejects_invalid_sort_parameter() {
    let app = spawn_app().await;

    let response = app.get_all_posts("?sort=invalid").await;
    assert_eq!(
        response.status().as_u16(),
        400,
        "Expected 400 for invalid sort parameter"
    );
}

#[tokio::test]
async fn get_all_posts_rejects_title_exceeding_maximum_length() {
    let app = spawn_app().await;

    let long_title = "a".repeat(101);
    let response = app.get_all_posts(&format!("?title={long_title}")).await;
    assert_eq!(
        response.status().as_u16(),
        400,
        "Expected 400 for title > 100 characters"
    );
}

// ============================================================================
// Sorting Tests
// ============================================================================

#[tokio::test]
async fn get_all_posts_sorts_by_id_descending_by_default() {
    let app = spawn_app().await;
    app.login().await;

    let id1 = create_sample_post(&app, "First", "Content").await;
    let id2 = create_sample_post(&app, "Second", "Content").await;
    let id3 = create_sample_post(&app, "Third", "Content").await;

    let response = app.get_all_posts("").await;
    assert_eq!(response.status().as_u16(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    let posts = body["posts"].as_array().unwrap();

    // Default sort is -id, so the newest first
    assert_eq!(posts[0]["id"], id3.to_string());
    assert_eq!(posts[1]["id"], id2.to_string());
    assert_eq!(posts[2]["id"], id1.to_string());
}

#[tokio::test]
async fn get_all_posts_sorts_by_created_at_descending_by_default() {
    let app = spawn_app().await;
    app.login().await;

    let id1 = create_sample_post(&app, "First", "Content").await;
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    let id2 = create_sample_post(&app, "Second", "Content").await;
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    let id3 = create_sample_post(&app, "Third", "Content").await;

    let response = app.get_all_posts("").await;
    assert_eq!(response.status().as_u16(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    let posts = body["posts"].as_array().unwrap();

    // Default sort is -created_at, so the newest first
    assert_eq!(posts[0]["id"], id3.to_string());
    assert_eq!(posts[1]["id"], id2.to_string());
    assert_eq!(posts[2]["id"], id1.to_string());
}

#[tokio::test]
async fn get_all_posts_sorts_by_title_ascending() {
    let app = spawn_app().await;
    app.login().await;

    create_sample_post(&app, "Zebra", "Content").await;
    create_sample_post(&app, "Apple", "Content").await;
    create_sample_post(&app, "Banana", "Content").await;

    let response = app.get_all_posts("?sort=title").await;
    assert_eq!(response.status().as_u16(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    let posts = body["posts"].as_array().unwrap();

    assert_eq!(posts[0]["title"], "Apple");
    assert_eq!(posts[1]["title"], "Banana");
    assert_eq!(posts[2]["title"], "Zebra");
}

#[tokio::test]
async fn get_all_posts_sorts_by_title_descending() {
    let app = spawn_app().await;
    app.login().await;

    create_sample_post(&app, "Zebra", "Content").await;
    create_sample_post(&app, "Apple", "Content").await;
    create_sample_post(&app, "Banana", "Content").await;

    let response = app.get_all_posts("?sort=-title").await;
    assert_eq!(response.status().as_u16(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    let posts = body["posts"].as_array().unwrap();

    assert_eq!(posts[0]["title"], "Zebra");
    assert_eq!(posts[1]["title"], "Banana");
    assert_eq!(posts[2]["title"], "Apple");
}

#[tokio::test]
async fn get_all_posts_sorts_by_likes_count_descending() {
    let app = spawn_app().await;
    app.login().await;

    create_sample_post(&app, "Post 1", "Content").await;
    let post2 = create_sample_post(&app, "Post 2", "Content").await;
    let post3 = create_sample_post(&app, "Post 3", "Content").await;

    // Like post2 twice (need different users or same user liking once)
    like_post_as_user(&app, &post2).await;

    // Like post3 once
    like_post_as_user(&app, &post3).await;

    // post1 has 0 likes

    let response = app.get_all_posts("?sort=-likescount").await;
    assert_eq!(response.status().as_u16(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    let posts = body["posts"].as_array().unwrap();

    // Should be ordered by likes: post2 (1 like), post3 (1 like), post1 (0 likes)
    // Posts with same like count will be ordered by id
    assert!(
        posts[0]["liked_by"].as_array().unwrap().len()
            >= posts[1]["liked_by"].as_array().unwrap().len()
    );
    assert!(
        posts[1]["liked_by"].as_array().unwrap().len()
            >= posts[2]["liked_by"].as_array().unwrap().len()
    );
}

// ============================================================================
// Title Search Tests
// ============================================================================

#[tokio::test]
async fn get_all_posts_filters_by_title() {
    let app = spawn_app().await;
    app.login().await;

    create_sample_post(&app, "Rust Programming", "Content").await;
    create_sample_post(&app, "Python Tutorial", "Content").await;
    create_sample_post(&app, "Rust Best Practices", "Content").await;

    let response = app.get_all_posts("?title=Rust").await;
    assert_eq!(response.status().as_u16(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    let posts = body["posts"].as_array().unwrap();

    assert_eq!(posts.len(), 2, "Expected 2 posts matching 'Rust'");
    assert_eq!(body["metadata"]["total_records"], 2);
}

#[tokio::test]
async fn get_all_posts_title_search_is_case_insensitive() {
    let app = spawn_app().await;
    app.login().await;

    create_sample_post(&app, "JavaScript Tutorial", "Content").await;
    create_sample_post(&app, "Python Guide", "Content").await;

    let response = app.get_all_posts("?title=javascript").await;
    assert_eq!(response.status().as_u16(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    let posts = body["posts"].as_array().unwrap();

    assert_eq!(posts.len(), 1);
    assert!(posts[0]["title"].as_str().unwrap().contains("JavaScript"));
}

#[tokio::test]
async fn get_all_posts_returns_all_posts_when_title_is_empty() {
    let app = spawn_app().await;
    app.login().await;

    create_sample_post(&app, "Post 1", "Content").await;
    create_sample_post(&app, "Post 2", "Content").await;

    let response = app.get_all_posts("?title=").await;
    assert_eq!(response.status().as_u16(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["posts"].as_array().unwrap().len(), 2);
}

// ============================================================================
// Filter by Creator Tests
// ============================================================================

#[tokio::test]
async fn get_all_posts_filters_by_creator_id() {
    let app = spawn_app().await;
    app.login().await;

    let creator_id = app.test_user.user_id;

    create_sample_post(&app, "My Post 1", "Content").await;
    create_sample_post(&app, "My Post 2", "Content").await;

    let response = app.get_all_posts(&format!("?id={creator_id}")).await;
    assert_eq!(response.status().as_u16(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    let posts = body["posts"].as_array().unwrap();

    assert_eq!(posts.len(), 2);

    // Verify all posts are created by the specified user
    for post in posts {
        assert_eq!(post["created_by"], creator_id.to_string());
    }
}

#[tokio::test]
async fn get_all_posts_returns_empty_for_nonexistent_creator() {
    let app = spawn_app().await;
    app.login().await;

    create_sample_post(&app, "Some Post", "Content").await;

    let random_id = Uuid::new_v4();
    let response = app.get_all_posts(&format!("?id={random_id}")).await;
    assert_eq!(response.status().as_u16(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["posts"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn get_all_posts_returns_all_posts_when_id_is_empty() {
    let app = spawn_app().await;
    app.login().await;

    create_sample_post(&app, "Post 1", "Content").await;
    create_sample_post(&app, "Post 2", "Content").await;

    let response = app.get_all_posts("?id=").await;
    assert_eq!(response.status().as_u16(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["posts"].as_array().unwrap().len(), 2);
}

// ============================================================================
// Soft Delete Tests
// ============================================================================

#[tokio::test]
async fn get_all_posts_excludes_soft_deleted_posts() {
    let app = spawn_app().await;
    app.login().await;

    let post1 = create_sample_post(&app, "Active Post", "Content").await;
    let post2 = create_sample_post(&app, "Deleted Post", "Content").await;

    // Soft delete post2
    app.delete_post(&post2).await;

    let response = app.get_all_posts("").await;
    assert_eq!(response.status().as_u16(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    let posts = body["posts"].as_array().unwrap();

    assert_eq!(posts.len(), 1, "Expected only 1 active post");
    assert_eq!(posts[0]["id"], post1.to_string());
}

// ============================================================================
// Response Structure Tests
// ============================================================================

#[tokio::test]
async fn get_all_posts_returns_correct_post_structure() {
    let app = spawn_app().await;
    app.login().await;

    create_sample_post(&app, "Test Post", "Test Content").await;

    let response = app.get_all_posts("").await;
    assert_eq!(response.status().as_u16(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    let post = &body["posts"][0];

    // Verify all expected fields exist
    assert!(post["id"].is_string());
    assert!(post["title"].is_string());
    assert!(post["text"].is_string());
    assert!(post["img"].is_string());
    assert!(post["version"].is_number());
    assert!(post["created_at"].is_string());
    assert!(post["created_by"].is_string());
    assert!(post["liked_by"].is_array());
}

// ============================================================================
// Combined Filters Tests
// ============================================================================

#[tokio::test]
async fn get_all_posts_combines_title_and_creator_filters() {
    let app = spawn_app().await;
    app.login().await;

    let creator_id = app.test_user.user_id;

    create_sample_post(&app, "Rust Tutorial", "Content").await;
    create_sample_post(&app, "Python Guide", "Content").await;
    create_sample_post(&app, "Rust Advanced", "Content").await;

    let response = app
        .get_all_posts(&format!("?title=Rust&id={creator_id}"))
        .await;
    assert_eq!(response.status().as_u16(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    let posts = body["posts"].as_array().unwrap();

    assert_eq!(posts.len(), 2, "Expected 2 Rust posts by this creator");
    assert_eq!(body["metadata"]["total_records"], 2);
}

#[tokio::test]
async fn get_all_posts_combines_filters_with_pagination_and_sorting() {
    let app = spawn_app().await;
    app.login().await;

    create_sample_post(&app, "Apple Tutorial", "Content").await;
    create_sample_post(&app, "Banana Guide", "Content").await;
    create_sample_post(&app, "Cherry Advanced", "Content").await;

    let response = app.get_all_posts("?sort=title&limit=2&page=1").await;
    assert_eq!(response.status().as_u16(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    let posts = body["posts"].as_array().unwrap();

    assert_eq!(posts.len(), 2);
    assert_eq!(posts[0]["title"], "Apple Tutorial");
    assert_eq!(posts[1]["title"], "Banana Guide");
    assert_eq!(body["metadata"]["total_records"], 3);
}
