use crate::helpers::spawn_app;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn the_link_sent_by_send_subscribe_email_returns_a_200_if_called() {
    let app = spawn_app().await;
    app.login().await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.send_subscribe_email().await;

    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = app.get_confirmation_links(email_request);

    let response = reqwest::get(confirmation_links.html).await.unwrap();
    assert_eq!(response.status().as_u16(), 200);
}

#[tokio::test]
async fn clicking_on_the_confirm_subscription_link_subscribes_a_user_in_db() {
    let app = spawn_app().await;
    app.login().await;

    Mock::given(path("/email"))
        .and(method("POST"))
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
        WHERE name = $1
        "#,
        &app.test_user.username,
    )
    .fetch_one(&app.db_pool)
    .await
    .expect("Failed to fetch saved user data.");

    assert!(saved.is_activated);
    assert!(saved.is_subscribed);
}

#[tokio::test]
async fn subscribe_user_requests_without_token_are_rejected_with_a_400() {
    let app = spawn_app().await;

    let response = reqwest::get(&format!("{}/user/confirm/subscribe", app.address))
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 400);
}

#[tokio::test]
async fn subscribe_user_with_invalid_token_returns_401() {
    let app = spawn_app().await;

    let response = reqwest::get(&format!(
        "{}/user/confirm/subscribe?token=not-a-real-token",
        app.address
    ))
    .await
    .unwrap();
    assert_eq!(response.status().as_u16(), 401);
}

#[tokio::test]
async fn subscription_token_is_deleted_after_successful_subscription() {
    let app = spawn_app().await;
    app.login().await;

    Mock::given(path("/email"))
        .and(method("POST"))
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
    let app = spawn_app().await;
    app.login().await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&app.email_server)
        .await;

    let response = app.send_subscribe_email().await;
    assert_eq!(response.status().as_u16(), 500);
}
