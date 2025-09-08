use crate::helpers::spawn_app;
use wiremock::{ResponseTemplate, Mock};
use wiremock::matchers::{path, method};

#[tokio::test]
async fn the_link_returned_by_add_user_returns_a_200_if_called() {
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

    let response = reqwest::get(confirmation_links.html)
        .await
        .unwrap();

    assert_eq!(response.status().as_u16(), 200);
}

#[tokio::test]
async fn clicking_on_the_confirmation_link_activates_a_user_in_db() {
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

    let response = reqwest::get(confirmation_links.html)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();

    assert_eq!(response.status().as_u16(), 200);

    let saved = sqlx::query!("SELECT email, name, is_activated FROM users",)
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved user data.");
    assert_eq!(saved.email, "athfantest@gmail.com");
    assert_eq!(saved.name, "athfantest");
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