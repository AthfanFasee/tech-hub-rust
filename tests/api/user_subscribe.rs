use crate::helpers;
use wiremock::matchers;
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn subscription_link_via_email_subscribes_a_user() {
    let app = helpers::spawn_app().await;
    app.login().await;

    Mock::given(matchers::path("/email"))
        .and(matchers::method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.send_subscribe_email().await;

    // Stimulate that user will be clicking confirmation email outside our app by logging out
    app.logout().await;

    // Extract confirmation link and "click" it
    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = app.get_confirmation_links(email_request);

    let response = reqwest::get(confirmation_links.html)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    assert_eq!(response.status().as_u16(), 200);

    let saved = sqlx::query!(
        r#"
        SELECT is_activated, is_subscribed
        FROM users
        WHERE user_name = $1
        "#,
        &app.test_user.user_name,
    )
    .fetch_one(&app.db_pool)
    .await
    .expect("Failed to fetch saved user data.");

    assert!(saved.is_activated);
    assert!(saved.is_subscribed);
}

#[tokio::test]
async fn subscribe_user_returns_400_when_token_is_missing() {
    let app = helpers::spawn_app().await;

    let response = reqwest::get(&format!("{}/v1/user/confirm/subscribe", app.address))
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 400);
}

#[tokio::test]
async fn subscribe_user_returns_401_for_invalid_token() {
    let app = helpers::spawn_app().await;

    let response = reqwest::get(&format!(
        "{}/v1/user/confirm/subscribe?token=not-a-real-token",
        app.address
    ))
    .await
    .unwrap();
    assert_eq!(response.status().as_u16(), 401);
}

#[tokio::test]
async fn subscribe_user_deletes_subscription_token_after_successful_subscription() {
    let app = helpers::spawn_app().await;
    app.login().await;

    Mock::given(matchers::path("/email"))
        .and(matchers::method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.send_subscribe_email().await;

    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = app.get_confirmation_links(email_request);

    reqwest::get(confirmation_links.html).await.unwrap();

    let remaining_tokens = sqlx::query!(
        r#"SELECT COUNT(*) as count FROM tokens WHERE user_id = $1 AND is_subscription = true"#,
        app.test_user.user_id,
    )
    .fetch_one(&app.db_pool)
    .await
    .unwrap();

    assert_eq!(remaining_tokens.count, Some(0));
}

#[tokio::test]
async fn send_subscribe_email_returns_500_if_email_sending_fails() {
    let app = helpers::spawn_app().await;
    app.login().await;

    Mock::given(matchers::path("/email"))
        .and(matchers::method("POST"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&app.email_server)
        .await;

    let response = app.send_subscribe_email().await;
    assert_eq!(response.status().as_u16(), 500);
}
