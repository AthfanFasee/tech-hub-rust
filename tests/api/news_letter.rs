use crate::helpers::{ConfirmationLinks, TestApp, TestUser, spawn_app};
use std::time::Duration;
use techhub::newsletter_delivery_worker::cleanup_old_newsletter_issues;
use uuid::Uuid;
use wiremock::matchers::{any, method, path};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn newsletters_are_not_delivered_to_inactivated_user() {
    let app = spawn_app().await;
    create_inactivated_user(&app).await;
    app.login_admin().await;

    Mock::given(any())
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

    let key = uuid::Uuid::new_v4().to_string();
    let response = app.publish_newsletters(&newsletter_body, Some(&key)).await;
    assert_eq!(response.status().as_u16(), 200);

    app.dispatch_all_pending_newsletter_emails().await;
}

#[tokio::test]
async fn newsletters_are_not_delivered_to_confirmed_but_unsubscribed_users() {
    let app = spawn_app().await;
    create_activated_user(&app).await;
    app.login_admin().await;

    Mock::given(path("/email"))
        .and(method("POST"))
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

    let key = uuid::Uuid::new_v4().to_string();
    let response = app.publish_newsletters(&newsletter_body, Some(&key)).await;
    assert_eq!(response.status().as_u16(), 200);

    app.dispatch_all_pending_newsletter_emails().await;
}

#[tokio::test]
async fn newsletters_returns_400_for_invalid_data() {
    let app = spawn_app().await;
    app.login_admin().await;

    let invalid_cases = vec![
        (
            serde_json::json!({ "content": { "text": "Body", "html": "<p>HTML</p>" } }),
            "missing title",
        ),
        (
            serde_json::json!({ "title": "Newsletter!" }),
            "missing content",
        ),
    ];

    for (invalid_body, desc) in invalid_cases {
        let key = uuid::Uuid::new_v4().to_string();
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
async fn non_admins_are_rejected_to_publish_newsletters() {
    let app = spawn_app().await;
    app.login().await;

    let newsletter_body = serde_json::json!({
        "title": "Newsletter title",
        "content": {
            "text": "Plain text",
            "html": "<p>HTML</p>"
        }
    });

    let key = uuid::Uuid::new_v4().to_string();
    let response = app.publish_newsletters(&newsletter_body, Some(&key)).await;
    assert_eq!(response.status().as_u16(), 403);
}

#[tokio::test]
async fn anonymous_users_cannot_publish_newsletters() {
    let app = spawn_app().await;

    let newsletter_body = serde_json::json!({
        "title": "Unauthorized attempt",
        "content": {
            "text": "Plain text",
            "html": "<p>HTML</p>"
        }
    });

    let key = uuid::Uuid::new_v4().to_string();
    let response = app.publish_newsletters(&newsletter_body, Some(&key)).await;
    assert_eq!(response.status().as_u16(), 401);
}

#[tokio::test]
async fn newsletters_are_delivered_to_a_user_who_subscribed_via_the_full_flow() {
    let app = spawn_app().await;
    create_active_subscriber(&app).await;
    app.login_admin().await;

    Mock::given(path("/email"))
        .and(method("POST"))
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

    let key = uuid::Uuid::new_v4().to_string();
    let response = app.publish_newsletters(&newsletter_body, Some(&key)).await;
    assert_eq!(response.status().as_u16(), 200);

    app.dispatch_all_pending_newsletter_emails().await;
}

#[tokio::test]
async fn newsletter_publishing_is_idempotent() {
    let app = spawn_app().await;
    create_active_subscriber(&app).await;
    app.login_admin().await;

    Mock::given(path("/email"))
        .and(method("POST"))
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

    let key = uuid::Uuid::new_v4().to_string();
    // Stimulate publishing newsletters twice back to back
    let response = app.publish_newsletters(&newsletter_body, Some(&key)).await;
    assert_eq!(response.status().as_u16(), 200);
    let response = app.publish_newsletters(&newsletter_body, Some(&key)).await;
    assert_eq!(response.status().as_u16(), 200);

    app.dispatch_all_pending_newsletter_emails().await;
}

#[tokio::test]
async fn concurrent_newsletter_publishing_is_handled_gracefully() {
    let app = spawn_app().await;
    create_active_subscriber(&app).await;
    app.login_admin().await;

    Mock::given(path("/email"))
        .and(method("POST"))
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

    let key = uuid::Uuid::new_v4().to_string();
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
async fn failed_newsletter_delivery_is_retried_with_back_off() {
    let app = spawn_app().await;
    create_active_subscriber(&app).await;
    app.login_admin().await;

    Mock::given(path("/email"))
        .and(method("POST"))
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

    let key = uuid::Uuid::new_v4().to_string();
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
async fn old_newsletter_issues_are_cleaned_up() {
    let app = spawn_app().await;
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

    cleanup_old_newsletter_issues(pool).await.unwrap();

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

async fn create_inactivated_user(app: &TestApp) -> (serde_json::Value, ConfirmationLinks) {
    let user = TestUser::generate();
    let payload = serde_json::json!({
        "user_name": user.user_name,
        "email": user.email,
        "password": user.password,
    });

    let _mock_guard = Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .named("Create inactivated user")
        .expect(1)
        .mount_as_scoped(&app.email_server)
        .await;

    app.register_user(&payload)
        .await
        .error_for_status()
        .unwrap();

    let email_request = &app
        .email_server
        .received_requests()
        .await
        .unwrap()
        .pop()
        .unwrap();

    let confirmation_links = app.get_confirmation_links(email_request);
    (payload, confirmation_links)
}

async fn create_activated_user(app: &TestApp) -> serde_json::Value {
    let (payload, confirmation_link) = create_inactivated_user(app).await;
    reqwest::get(confirmation_link.html)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    payload
}

pub async fn create_active_subscriber(app: &TestApp) {
    let payload = create_activated_user(app).await;

    let response = app.login_with(&payload).await;
    assert_eq!(response.status().as_u16(), 200);

    let _mock_guard = Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .named("Subscription confirmation email")
        .expect(1)
        .mount_as_scoped(&app.email_server)
        .await;

    app.send_subscribe_email().await;

    // Stimulate that user will be clicking confirmation email outside our app by logging out
    app.logout().await;

    // Extract confirmation link from subscription email and "click" it
    let email_request = &app.email_server.received_requests().await.unwrap()[1];
    let confirmation_links = app.get_confirmation_links(email_request);
    reqwest::get(confirmation_links.html)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
}
