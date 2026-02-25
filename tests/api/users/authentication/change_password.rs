use uuid::Uuid;

use crate::helpers;

#[tokio::test]
async fn change_password_returns_401_for_unauthenticated_users() {
    let app = helpers::spawn_app().await;
    let new_password = Uuid::new_v4().to_string();

    let response = app
        .change_password(&serde_json::json!({
            "current_password": Uuid::new_v4().to_string(),
            "new_password": &new_password,
        }))
        .await;

    assert_eq!(
        401,
        response.status().as_u16(),
        "The API did not respond with 401 status upon unauthorized user trying to access it."
    );
}

#[tokio::test]
async fn change_password_returns_401_for_invalid_current_password() {
    let app = helpers::spawn_app().await;
    let new_password = Uuid::new_v4().to_string();
    let wrong_password = Uuid::new_v4().to_string();

    app.login().await;

    let response = app
        .change_password(&serde_json::json!({
        "current_password": &wrong_password,
        "new_password": &new_password,
        }))
        .await;

    assert_eq!(
        401,
        response.status().as_u16(),
        "The API did not respond with 401 status upon current password is wrong."
    );
}

#[tokio::test]
async fn change_password_changes_password_and_returns_200() {
    let app = helpers::spawn_app().await;
    let new_password = Uuid::new_v4().to_string();

    app.login().await;

    // Change password
    let response = app
        .change_password(&serde_json::json!({
        "current_password": &app.test_user.password,
        "new_password": &new_password,
        }))
        .await;
    assert_eq!(response.status().as_u16(), 200);

    // Logout
    let response = app.logout().await;
    assert_eq!(response.status().as_u16(), 200);

    //  Login using the new password
    let login_body = serde_json::json!({
    "user_name": &app.test_user.user_name,
    "password": &new_password
    });

    let response = app.login_with(&login_body).await;
    assert_eq!(response.status().as_u16(), 200);

    // Access protected endpoint
    let response = app.access_protected().await;
    assert_eq!(response.status().as_u16(), 200);
}
