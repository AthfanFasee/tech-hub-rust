use crate::helpers::spawn_app;
use uuid::Uuid;

#[tokio::test]
async fn login_returns_success_for_valid_username_and_password() {
    let app = spawn_app().await;

    let payload = serde_json::json!({
    "username": &app.test_user.username,
    "password": &app.test_user.password
    });

    let response = app.login_with(&payload).await;

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

    let response = app.login_with(&payload).await;

    assert_eq!(
        401,
        response.status().as_u16(),
        "The API did not respond with 401 status upon providing invalid username or password."
    );
}

#[tokio::test]
async fn login_does_not_leak_internal_error_details_on_auth_failure() {
    let app = spawn_app().await;

    // Use a valid username but wrong password
    let payload = serde_json::json!({
        "username": &app.test_user.username,
        "password": Uuid::new_v4().to_string(),
    });

    let response = app.login_with(&payload).await;
    assert_eq!(
        401,
        response.status().as_u16(),
        "Expected 401 for invalid credentials."
    );

    // response body should not leak details like 'Invalid password'
    let body = response.text().await.unwrap();
    assert!(
        !body.contains("Invalid password"),
        "Response leaked specific password-related message: {body}"
    );
    assert!(
        !body.contains("hash"),
        "Response leaked sensitive error details: {body}"
    );
}

#[tokio::test]
async fn logout_clears_session_state() {
    let app = spawn_app().await;
    app.login().await;

    let response = app.access_protected().await;
    assert_eq!(response.status().as_u16(), 200);

    let response = app.logout().await;
    assert_eq!(response.status().as_u16(), 200);

    // Try again to access protected endpoint
    let response = app.access_protected().await;
    assert_eq!(
        response.status().as_u16(),
        401,
        "Expected 401 Unauthorized after logout when accessing protected endpoint"
    );
}
