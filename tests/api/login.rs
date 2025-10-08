use crate::helpers::spawn_app;
use uuid::Uuid;

#[tokio::test]
async fn login_returns_success_for_valid_username_and_password() {
    // Arrange
    let app = spawn_app().await;

    // Act
    let payload = serde_json::json!({
    "username": &app.test_user.username,
    "password": &app.test_user.password
    });

    let response = app.login(&payload).await;

    // Assert
    assert_eq!(
        200,
        response.status().as_u16(),
        "The API did not succeed with 200 status upon providing a valid username and password."
    );
}

#[tokio::test]
async fn login_returns_unauthorized_for_invalid_username_or_password() {
    // Arrange
    let app = spawn_app().await;

    // Act
    let payload = serde_json::json!({
    "username": &app.test_user.username,
    "password": Uuid::new_v4().to_string()
    });

    let response = app.login(&payload).await;

    // Assert
    assert_eq!(
        401,
        response.status().as_u16(),
        "The API did not respond with 401 status upon providing invalid username or password."
    );
}
