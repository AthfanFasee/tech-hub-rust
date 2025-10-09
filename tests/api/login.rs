use crate::helpers::spawn_app;
use uuid::Uuid;

#[tokio::test]
async fn login_returns_success_for_valid_username_and_password() {
    let app = spawn_app().await;

    let payload = serde_json::json!({
    "username": &app.test_user.username,
    "password": &app.test_user.password
    });

    let response = app.login(&payload).await;

    assert_eq!(
        200,
        response.status().as_u16(),
        "The API did not succeed with 200 status upon providing a valid username and password."
    );
}

#[tokio::test]
async fn login_returns_unauthorized_for_invalid_username_or_password() {
    let app = spawn_app().await;

    let payload = serde_json::json!({
    "username": &app.test_user.username,
    "password": Uuid::new_v4().to_string()
    });

    let response = app.login(&payload).await;

    assert_eq!(
        401,
        response.status().as_u16(),
        "The API did not respond with 401 status upon providing invalid username or password."
    );
}

#[tokio::test]
async fn logout_clears_session_state() {
    let app = spawn_app().await;

    // Login
    let login_body = serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password
    });

    let response = app.login(&login_body).await;
    assert_eq!(response.status().as_u16(), 200);

    // Access protected endpoint
    let response = app.access_protected_endpoint().await;
    assert_eq!(response.status().as_u16(), 200);

    // Logout
    let response = app.logout().await;
    assert_eq!(response.status().as_u16(), 200);

    // Try again to access protected endpoint
    let response = app.access_protected_endpoint().await;
    assert_eq!(
        response.status().as_u16(),
        401,
        "Expected 401 Unauthorized after logout from protected endpoint"
    );
}
