use crate::helpers::spawn_app;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn add_user_returns_a_200_for_valid_json_data() {
    let app = spawn_app().await;

    let payload = serde_json::json!({
        "name": "athfantest",
        "email": "athfantest@gmail.com"
    });

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    let response = app.add_user(&payload).await;

    assert!(response.status().is_success());
}

#[tokio::test]
async fn add_user_persists_the_new_user() {
    let app = spawn_app().await;

    let payload = serde_json::json!({
        "name": "athfantest",
        "email": "athfantest@gmail.com"
    });

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.add_user(&payload).await;

    let saved = sqlx::query!("SELECT email, name, is_activated, is_subscribed FROM users",)
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved user data.");

    assert_eq!(saved.email, "athfantest@gmail.com");
    assert_eq!(saved.name, "athfantest");
    assert!(!saved.is_activated);
    assert!(!saved.is_subscribed);
}

#[tokio::test]
async fn add_user_returns_a_400_when_data_is_missing() {
    let app = spawn_app().await;

    let test_cases = vec![
        (serde_json::json!({ "name": "athfan" }), "missing the email"),
        (
            serde_json::json!({ "email": "athfantest@gmail.com" }),
            "missing the name",
        ),
        (serde_json::json!({}), "missing both name and email"),
    ];

    for (invalid_payload, _error_message) in test_cases {
        let response = app.add_user(&invalid_payload).await;

        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request when the payload was {_error_message}."
        );
    }
}

#[tokio::test]
async fn add_user_returns_a_400_when_data_is_present_but_invalid() {
    let app = spawn_app().await;

    let test_cases = vec![
        (
            serde_json::json!({ "name": "athfan", "email": "" }),
            "empty email string",
        ),
        (
            serde_json::json!({ "email": "athfantest@gmail.com", "name": "" }),
            "empty name string",
        ),
        (
            serde_json::json!({"name": "athfan", "email": "definitely wrong email"}),
            "invalid email address",
        ),
        (
            serde_json::json!({"name": "ath/fan)", "email": "athfantest@gmail.com"}),
            "name contains invalid characters",
        ),
    ];

    for (invalid_payload, _error_message) in test_cases {
        let response = app.add_user(&invalid_payload).await;

        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request when the payload was {_error_message}."
        );
    }
}

#[tokio::test]
async fn add_user_sends_a_confirmation_email_for_valid_data() {
    let app = spawn_app().await;
    let payload = serde_json::json!({
        "name": "athfantest",
        "email": "athfantest@gmail.com"
    });

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    app.add_user(&payload).await;
}

#[tokio::test]
async fn add_user_sends_a_confirmation_email_with_a_link() {
    let app = spawn_app().await;
    let payload = serde_json::json!({
        "name": "athfantest",
        "email": "athfantest@gmail.com"
    });

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.add_user(&payload).await;

    let email_request = &app.email_server.received_requests().await.unwrap()[0];

    let confirmation_links = app.get_confirmation_links(email_request);

    // The two links should be identical
    assert_eq!(confirmation_links.html, confirmation_links.plain_text);
}

#[tokio::test]
async fn add_user_fails_if_there_is_a_fatal_database_error() {
    let app = spawn_app().await;
    let payload = serde_json::json!({
        "name": "athfantest",
        "email": "athfantest@gmail.com"
    });

    // Sabotage the database
    sqlx::query!("ALTER TABLE tokens DROP COLUMN token;",)
        .execute(&app.db_pool)
        .await
        .unwrap();

    let response = app.add_user(&payload).await;

    assert_eq!(response.status().as_u16(), 500);
}
