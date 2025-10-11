use crate::helpers::{TestUser, spawn_app};
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn register_user_persists_the_new_user_and_returns_a_200_for_valid_json_data() {
    let app = spawn_app().await;

    let user = TestUser::generate();
    let payload = serde_json::json!({
        "name": user.username,
        "email": user.email,
        "password": user.password,
    });

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    let response = app.register_user(&payload).await;
    assert!(response.status().is_success());

    let saved = sqlx::query!(
        r#"
        SELECT email, name, is_activated, is_subscribed 
        FROM users
        WHERE email = $1
        "#,
        user.email,
    )
    .fetch_one(&app.db_pool)
    .await
    .expect("Failed to fetch saved user data.");

    assert_eq!(saved.email, user.email);
    assert_eq!(saved.name, user.username);
    assert!(!saved.is_activated);
    assert!(!saved.is_subscribed);
}

#[tokio::test]
async fn register_user_can_login_using_their_credentials() {
    let app = spawn_app().await;
    let user = TestUser::generate();
    let payload = serde_json::json!({
        "name": user.username,
        "email": user.email,
        "password": user.password,
    });

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    app.register_user(&payload).await;

    // Extract confirmation link and "click" it to activate user account
    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = app.get_confirmation_links(email_request);
    reqwest::get(confirmation_links.html)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();

    // login using the same credentials used when registering
    let payload = serde_json::json!({
    "username": user.username,
    "password": user.password
    });

    let response = app.login_with(&payload).await;
    assert!(response.status().is_success());

    // access a protected route to confirm user is successfully logged in
    let response = app.access_protected().await;
    assert!(response.status().is_success());
}

#[tokio::test]
async fn register_user_returns_a_400_when_data_is_missing() {
    let app = spawn_app().await;

    let user = TestUser::generate();
    let test_cases = vec![
        (
            serde_json::json!({ "name": user.username, "password": user.password }),
            "missing the email",
        ),
        (
            serde_json::json!({ "email": user.email, "password": user.password }),
            "missing the name",
        ),
        (
            serde_json::json!({ "name": user.username, "email": user.email }),
            "missing the password",
        ),
        (serde_json::json!({}), "missing name, email and password"),
    ];

    for (invalid_payload, _error_message) in test_cases {
        let response = app.register_user(&invalid_payload).await;
        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request when the payload was {_error_message}."
        );
    }
}

#[tokio::test]
async fn register_user_returns_a_400_when_data_is_present_but_invalid() {
    let app = spawn_app().await;

    let user = TestUser::generate();
    let test_cases = vec![
        (
            serde_json::json!({ "name": user.username, "email": "", "password": user.password }),
            "empty email string",
        ),
        (
            serde_json::json!({ "email": user.email, "name": "",  "password": user.password }),
            "empty name string",
        ),
        (
            serde_json::json!({"name": user.username, "email": "definitely wrong email", "password": user.password}),
            "invalid email address",
        ),
        (
            serde_json::json!({"name": "ath/fan)", "email": user.email, "password": user.password}),
            "name contains invalid characters",
        ),
        (
            serde_json::json!({"name": user.username, "email": user.email, "password": "123"}),
            "password too small",
        ),
    ];

    for (invalid_payload, _error_message) in test_cases {
        let response = app.register_user(&invalid_payload).await;
        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request when the payload was {_error_message}."
        );
    }
}

#[tokio::test]
async fn register_user_sends_a_confirmation_email_with_a_link() {
    let app = spawn_app().await;
    let user = TestUser::generate();
    let payload = serde_json::json!({
        "name": user.username,
        "email": user.email,
        "password": user.password,
    });

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    app.register_user(&payload).await;

    let email_request = &app.email_server.received_requests().await.unwrap()[0];

    let confirmation_links = app.get_confirmation_links(email_request);

    // The two links should be identical
    assert_eq!(confirmation_links.html, confirmation_links.plain_text);
}

#[tokio::test]
async fn register_user_fails_if_there_is_a_fatal_database_error() {
    let app = spawn_app().await;
    let user = TestUser::generate();
    let payload = serde_json::json!({
        "name": user.username,
        "email": user.email,
         "password": user.password,
    });

    // Sabotage the database
    sqlx::query!("ALTER TABLE tokens DROP COLUMN token;",)
        .execute(&app.db_pool)
        .await
        .unwrap();

    let response = app.register_user(&payload).await;
    assert_eq!(response.status().as_u16(), 500);
}
