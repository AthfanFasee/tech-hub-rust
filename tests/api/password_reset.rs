use crate::helpers::spawn_app;
use uuid::Uuid;

#[tokio::test]
async fn user_must_be_logged_in_to_change_your_password() {
    let app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();

    let response = app
        .change_password(&serde_json::json!({
            "current_password": Uuid::new_v4().to_string(),
            "new_password": &new_password,
            "new_password_check": &new_password,
        }))
        .await;

    assert_eq!(
        401,
        response.status().as_u16(),
        "The API did not respond with 401 status upon unauthorized user trying to access it."
    );
}

#[tokio::test]
async fn new_password_fields_must_match() {
    let app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();
    let another_new_password = Uuid::new_v4().to_string();

    app.login(&serde_json::json!({
    "username": &app.test_user.username,
    "password": &app.test_user.password
    }))
    .await;

    let response = app
        .change_password(&serde_json::json!({
        "current_password": &app.test_user.password,
        "new_password": &new_password,
        "new_password_check": &another_new_password,
        }))
        .await;

    assert_eq!(
        400,
        response.status().as_u16(),
        "The API did not respond with 400 status upon passwords did not match."
    );
}

#[tokio::test]
async fn current_password_must_be_valid() {
    let app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();
    let wrong_password = Uuid::new_v4().to_string();

    app.login(&serde_json::json!({
    "username": &app.test_user.username,
    "password": &app.test_user.password
    }))
    .await;

    let response = app
        .change_password(&serde_json::json!({
        "current_password": &wrong_password,
        "new_password": &new_password,
        "new_password_check": &new_password,
        }))
        .await;

    assert_eq!(
        401,
        response.status().as_u16(),
        "The API did not respond with 401 status upon current password is wrong."
    );
}
