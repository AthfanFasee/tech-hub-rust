use crate::helpers::{TestUser, spawn_app};
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn the_link_returned_by_add_user_returns_a_200_if_called() {
    let app = spawn_app().await;
    let user = TestUser::generate();
    let payload = serde_json::json!({
        "name": user.username,
        "email": user.email
    });

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.register_user(&payload).await;

    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = app.get_confirmation_links(email_request);

    let response = reqwest::get(confirmation_links.html).await.unwrap();

    assert_eq!(response.status().as_u16(), 200);
}

#[tokio::test]
async fn clicking_on_the_confirmation_link_activates_a_user_in_db() {
    let app = spawn_app().await;
    let user = TestUser::generate();
    let payload = serde_json::json!({
        "name": user.username,
        "email": user.email
    });

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.register_user(&payload).await;

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
    assert!(saved.is_activated);
}

#[tokio::test]
async fn confirmations_without_token_are_rejected_with_a_400() {
    let app = spawn_app().await;

    let response = reqwest::get(&format!("{}/user/confirm", app.address))
        .await
        .unwrap();

    assert_eq!(response.status().as_u16(), 400);
}
