use crate::helpers;
use std::time::Duration;
use techhub::newsletter_delivery_worker;
use uuid::Uuid;
use wiremock::matchers;
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn publish_newsletter_does_not_deliver_to_inactivated_user() {
    let app = helpers::spawn_app().await;
    app.create_inactivated_user().await;
    app.login_admin().await;

    Mock::given(matchers::any())
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&app.email_server)
        .await;

    let newsletter_body = serde_json::json!({
        "title": "Newsletter title",
        "content": {
            "text": "Plain text",
            "html": "<p>HTML</p>"
        }
    });

    let key = Uuid::new_v4().to_string();
    let response = app.publish_newsletters(&newsletter_body, Some(&key)).await;
    assert_eq!(response.status().as_u16(), 200);

    app.dispatch_all_pending_newsletter_emails().await;
}

#[tokio::test]
async fn publish_newsletter_does_not_deliver_to_activated_but_unsubscribed_users() {
    let app = helpers::spawn_app().await;
    app.create_activated_user().await;
    app.login_admin().await;

    Mock::given(matchers::path("/email"))
        .and(matchers::method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&app.email_server)
        .await;

    let newsletter_body = serde_json::json!({
        "title": "Newsletter title",
        "content": {
            "text": "Plain text",
            "html": "<p>HTML</p>"
        }
    });

    let key = Uuid::new_v4().to_string();
    let response = app.publish_newsletters(&newsletter_body, Some(&key)).await;
    assert_eq!(response.status().as_u16(), 200);

    app.dispatch_all_pending_newsletter_emails().await;
}

#[tokio::test]
async fn publish_newsletter_returns_400_for_invalid_data() {
    let app = helpers::spawn_app().await;
    app.login_admin().await;

    let invalid_cases = vec![
        // Missing fields
        (
            serde_json::json!({
                "content": {
                    "text": "Body",
                    "html": "<p>HTML</p>"
                }
            }),
            "missing title",
        ),
        (
            serde_json::json!({
                "title": "Newsletter!"
            }),
            "missing content",
        ),
        (
            serde_json::json!({
                "title": "Newsletter!",
                "content": {
                    "html": "<p>HTML</p>"
                }
            }),
            "missing content text",
        ),
        (
            serde_json::json!({
                "title": "Newsletter!",
                "content": {
                    "text": "Body"
                }
            }),
            "missing content html",
        ),
        // Empty fields
        (
            serde_json::json!({
                "title": "",
                "content": {
                    "text": "Body",
                    "html": "<p>HTML</p>"
                }
            }),
            "empty title",
        ),
        (
            serde_json::json!({
                "title": "Newsletter!",
                "content": {
                    "text": "",
                    "html": "<p>HTML</p>"
                }
            }),
            "empty text",
        ),
        (
            serde_json::json!({
                "title": "Newsletter!",
                "content": {
                    "text": "Body",
                    "html": ""
                }
            }),
            "empty html",
        ),
        // Whitespace-only fields
        (
            serde_json::json!({
                "title": "   ",
                "content": {
                    "text": "Body",
                    "html": "<p>HTML</p>"
                }
            }),
            "whitespace-only title",
        ),
        (
            serde_json::json!({
                "title": "Newsletter!",
                "content": {
                    "text": "   \n\t   ",
                    "html": "<p>HTML</p>"
                }
            }),
            "whitespace-only text",
        ),
        (
            serde_json::json!({
                "title": "Newsletter!",
                "content": {
                    "text": "Body",
                    "html": "   \n\t   "
                }
            }),
            "whitespace-only html",
        ),
        // Title with only numbers
        (
            serde_json::json!({
                "title": "12345",
                "content": {
                    "text": "Body",
                    "html": "<p>HTML</p>"
                }
            }),
            "title with only numbers",
        ),
        (
            serde_json::json!({
                "title": "123 456",
                "content": {
                    "text": "Body",
                    "html": "<p>HTML</p>"
                }
            }),
            "title with only numbers and spaces",
        ),
        // Title too long
        (
            serde_json::json!({
                "title": "a".repeat(201),
                "content": {
                    "text": "Body",
                    "html": "<p>HTML</p>"
                }
            }),
            "title exceeding 200 characters",
        ),
        // Text too long
        (
            serde_json::json!({
                "title": "Newsletter!",
                "content": {
                    "text": "a".repeat(50_001),
                    "html": "<p>HTML</p>"
                }
            }),
            "text exceeding 50,000 characters",
        ),
        // HTML too long
        (
            serde_json::json!({
                "title": "Newsletter!",
                "content": {
                    "text": "Body",
                    "html": format!("<p>{}</p>", "a".repeat(100_000))
                }
            }),
            "html exceeding 100,000 characters",
        ),
        // Invalid HTML (plain text without tags)
        (
            serde_json::json!({
                "title": "Newsletter!",
                "content": {
                    "text": "Body",
                    "html": "This is just plain text without any HTML tags"
                }
            }),
            "html without valid tags",
        ),
    ];

    for (invalid_body, desc) in invalid_cases {
        let key = Uuid::new_v4().to_string();
        let response = app.publish_newsletters(&invalid_body, Some(&key)).await;
        assert_eq!(
            400,
            response.status().as_u16(),
            "Did not return 400 when payload was {desc}"
        );
    }

    app.dispatch_all_pending_newsletter_emails().await;
}

#[tokio::test]
async fn publish_newsletter_returns_200_for_valid_data() {
    let app = helpers::spawn_app().await;
    app.login_admin().await;

    let valid_cases = vec![
        (
            serde_json::json!({
                "title": "Weekly Newsletter",
                "content": {
                    "text": "This is the plain text version.",
                    "html": "<p>This is the HTML version.</p>"
                }
            }),
            "basic valid newsletter",
        ),
        (
            serde_json::json!({
                "title": "Newsletter 2025",
                "content": {
                    "text": "Plain text content here.",
                    "html": "<html><body><h1>Title</h1><p>Content</p></body></html>"
                }
            }),
            "newsletter with alphanumeric title",
        ),
        (
            serde_json::json!({
                "title": "📧 Newsletter Update!",
                "content": {
                    "text": "Plain text content.",
                    "html": "<div><p>HTML content</p></div>"
                }
            }),
            "newsletter with unicode in title",
        ),
        (
            serde_json::json!({
                "title": "a".repeat(200),
                "content": {
                    "text": "Plain text.",
                    "html": "<p>HTML</p>"
                }
            }),
            "newsletter with max length title",
        ),
        (
            serde_json::json!({
                "title": "Newsletter",
                "content": {
                    "text": "a".repeat(50_000),
                    "html": "<p>HTML</p>"
                }
            }),
            "newsletter with max length text",
        ),
        (
            serde_json::json!({
                "title": "Newsletter",
                "content": {
                    "text": "Plain text.",
                    "html": format!("<p>{}</p>", "a".repeat(99_990))
                }
            }),
            "newsletter with max length html",
        ),
    ];

    for (valid_body, desc) in valid_cases {
        let key = Uuid::new_v4().to_string();
        let response = app.publish_newsletters(&valid_body, Some(&key)).await;
        assert_eq!(
            200,
            response.status().as_u16(),
            "Did not return 200 when payload was {desc}"
        );
    }

    app.dispatch_all_pending_newsletter_emails().await;
}

#[tokio::test]
async fn publish_newsletter_returns_403_for_non_admins() {
    let app = helpers::spawn_app().await;
    app.login().await;

    let newsletter_body = serde_json::json!({
        "title": "Newsletter title",
        "content": {
            "text": "Plain text",
            "html": "<p>HTML</p>"
        }
    });

    let key = Uuid::new_v4().to_string();
    let response = app.publish_newsletters(&newsletter_body, Some(&key)).await;
    assert_eq!(response.status().as_u16(), 403);
}

#[tokio::test]
async fn publish_newsletter_returns_401_for_anonymous_users() {
    let app = helpers::spawn_app().await;

    let newsletter_body = serde_json::json!({
        "title": "Unauthorized attempt",
        "content": {
            "text": "Plain text",
            "html": "<p>HTML</p>"
        }
    });

    let key = Uuid::new_v4().to_string();
    let response = app.publish_newsletters(&newsletter_body, Some(&key)).await;
    assert_eq!(response.status().as_u16(), 401);
}

#[tokio::test]
async fn publish_newsletter_delivers_to_active_subscriber_full_flow() {
    let app = helpers::spawn_app().await;
    app.create_active_subscriber().await;
    app.login_admin().await;

    Mock::given(matchers::path("/email"))
        .and(matchers::method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let newsletter_body = serde_json::json!({
        "title": "Test Newsletter",
        "content": {
            "text": "Hello subscribers!",
            "html": "<p>Hello subscribers!</p>"
        }
    });

    let key = Uuid::new_v4().to_string();
    let response = app.publish_newsletters(&newsletter_body, Some(&key)).await;
    assert_eq!(response.status().as_u16(), 200);

    app.dispatch_all_pending_newsletter_emails().await;
}

#[tokio::test]
async fn publish_newsletter_is_idempotent() {
    let app = helpers::spawn_app().await;
    app.create_active_subscriber().await;
    app.login_admin().await;

    Mock::given(matchers::path("/email"))
        .and(matchers::method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let newsletter_body = serde_json::json!({
        "title": "Test Newsletter",
        "content": {
            "text": "Hello subscribers!",
            "html": "<p>Hello subscribers!</p>"
        }
    });

    let key = Uuid::new_v4().to_string();
    // Stimulate publishing newsletters twice back to back
    let response = app.publish_newsletters(&newsletter_body, Some(&key)).await;
    assert_eq!(response.status().as_u16(), 200);
    let response = app.publish_newsletters(&newsletter_body, Some(&key)).await;
    assert_eq!(response.status().as_u16(), 200);

    app.dispatch_all_pending_newsletter_emails().await;
}

#[tokio::test]
async fn publish_newsletter_handles_concurrent_requests_gracefully() {
    let app = helpers::spawn_app().await;
    app.create_active_subscriber().await;
    app.login_admin().await;

    Mock::given(matchers::path("/email"))
        .and(matchers::method("POST"))
        // Setting a delay ensures that the second request arrives before the first one completes
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(1)))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let newsletter_body = serde_json::json!({
        "title": "Test Newsletter",
        "content": {
            "text": "Hello subscribers!",
            "html": "<p>Hello subscribers!</p>"
        }
    });

    let key = Uuid::new_v4().to_string();
    let response1 = app.publish_newsletters(&newsletter_body, Some(&key));
    let response2 = app.publish_newsletters(&newsletter_body, Some(&key));

    // Stimulate publishing newsletters concurrently
    let (response1, response2) = tokio::join!(response1, response2);

    assert_eq!(response1.status(), response2.status());
    assert_eq!(
        response1.text().await.unwrap(),
        response2.text().await.unwrap()
    );

    app.dispatch_all_pending_newsletter_emails().await;
}

#[tokio::test]
async fn publish_newsletter_retries_failed_delivery_with_back_off() {
    let app = helpers::spawn_app().await;
    app.create_active_subscriber().await;
    app.login_admin().await;

    Mock::given(matchers::path("/email"))
        .and(matchers::method("POST"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&app.email_server)
        .await;

    let newsletter_body = serde_json::json!({
        "title": "Test Newsletter",
        "content": {
            "text": "Hello subscribers!",
            "html": "<p>Hello subscribers!</p>"
        }
    });

    let key = Uuid::new_v4().to_string();
    let response = app.publish_newsletters(&newsletter_body, Some(&key)).await;
    assert_eq!(response.status().as_u16(), 200);

    // Fetch the single delivery task created
    let tasks = sqlx::query!(
        r#"
        SELECT newsletter_issue_id, user_email, n_retries, execute_after
        FROM issue_delivery_queue
        "#,
    )
    .fetch_all(&app.db_pool)
    .await
    .expect("Expected to query issue_delivery_queue");

    assert_eq!(tasks.len(), 1, "Expected exactly one delivery task");
    let task = &tasks[0];

    app.dispatch_all_pending_newsletter_emails().await;

    // Assert that record still exists after failure, retry count incremented, execute_after is set to future
    let record = sqlx::query!(
        r#"
        SELECT n_retries, execute_after
        FROM issue_delivery_queue
        WHERE newsletter_issue_id = $1 AND user_email = $2
        "#,
        task.newsletter_issue_id,
        task.user_email
    )
    .fetch_one(&app.db_pool)
    .await
    .expect("Expected task to still exist after retry");

    assert_eq!(record.n_retries, 1, "Retry count should increment");
    assert!(
        record.execute_after > chrono::Utc::now(),
        "execute_after should be in the future"
    );
}

#[tokio::test]
async fn cleanup_old_newsletter_issues_deletes_issues_older_than_7_days() {
    let app = helpers::spawn_app().await;
    let pool = &app.db_pool;

    // Insert an old newsletter issue (older than 7 days)
    sqlx::query!(
        r#"
        INSERT INTO newsletter_issues (id, title, text_content, html_content, created_at)
        VALUES ($1, $2, $3, $4, NOW() - INTERVAL '8 days')
        "#,
        Uuid::new_v4(),
        "Old newsletter",
        "Old text content",
        "<p>Old HTML content</p>",
    )
    .execute(pool)
    .await
    .unwrap();

    // Insert a recent newsletter issue (newer than 7 days)
    let new_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO newsletter_issues (id, title, text_content, html_content, created_at)
        VALUES ($1, $2, $3, $4, NOW() - INTERVAL '2 days')
        "#,
        new_id,
        "Recent newsletter",
        "Recent text content",
        "<p>Recent HTML content</p>",
    )
    .execute(pool)
    .await
    .unwrap();

    newsletter_delivery_worker::cleanup_old_newsletter_issues(pool)
        .await
        .unwrap();

    // Old newsletter should be deleted
    let old_exists = sqlx::query_scalar!(
        r#"SELECT EXISTS(SELECT 1 FROM newsletter_issues WHERE title = $1)"#,
        "Old newsletter"
    )
    .fetch_one(pool)
    .await
    .unwrap()
    .unwrap();
    assert!(!old_exists, "Old newsletter issue was not deleted");

    // Recent newsletter should still exist
    let new_exists = sqlx::query_scalar!(
        r#"SELECT EXISTS(SELECT 1 FROM newsletter_issues WHERE id = $1)"#,
        new_id
    )
    .fetch_one(pool)
    .await
    .unwrap()
    .unwrap();
    assert!(new_exists, "Recent newsletter issue was wrongly deleted");
}
