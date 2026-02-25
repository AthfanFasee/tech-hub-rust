use uuid::Uuid;

use crate::helpers;

#[tokio::test]
async fn login_returns_success_for_valid_username_and_password() {
    let app = helpers::spawn_app().await;

    let payload = serde_json::json!({
    "user_name": &app.test_user.user_name,
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
    let app = helpers::spawn_app().await;

    let payload = serde_json::json!({
    "user_name": &app.test_user.user_name,
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
    let app = helpers::spawn_app().await;

    // Use a valid username but wrong password
    let payload = serde_json::json!({
        "user_name": &app.test_user.user_name,
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
    let app = helpers::spawn_app().await;
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

#[tokio::test]
async fn login_rejects_empty_username() {
    let app = helpers::spawn_app().await;

    let payload = serde_json::json!({
        "user_name": "",
        "password": &app.test_user.password
    });

    let response = app.login_with(&payload).await;

    assert_eq!(
        401,
        response.status().as_u16(),
        "Expected 401 for empty username due to validation failure"
    );
}

#[tokio::test]
async fn login_rejects_empty_password() {
    let app = helpers::spawn_app().await;

    let payload = serde_json::json!({
        "user_name": &app.test_user.user_name,
        "password": ""
    });

    let response = app.login_with(&payload).await;

    assert_eq!(
        401,
        response.status().as_u16(),
        "Expected 401 for empty password due to validation failure"
    );
}

#[tokio::test]
async fn login_rejects_username_that_is_too_long() {
    let app = helpers::spawn_app().await;

    let too_long_username = "a".repeat(257);

    let payload = serde_json::json!({
        "user_name": too_long_username,
        "password": &app.test_user.password
    });

    let response = app.login_with(&payload).await;

    assert_eq!(
        401,
        response.status().as_u16(),
        "Expected 401 for username exceeding maximum length"
    );
}

#[tokio::test]
async fn login_rejects_password_that_is_too_short() {
    let app = helpers::spawn_app().await;

    let payload = serde_json::json!({
        "user_name": &app.test_user.user_name,
        "password": "short"
    });

    let response = app.login_with(&payload).await;

    assert_eq!(
        401,
        response.status().as_u16(),
        "Expected 401 for password that doesn't meet minimum length requirement"
    );
}

#[tokio::test]
async fn login_rejects_username_with_forbidden_characters() {
    let app = helpers::spawn_app().await;

    // Test with characters that UserName validation should reject
    let invalid_usernames = vec![
        "user//name",
        "user<script>",
        "user\\name",
        "user{name}",
        "user(test)",
        r#"user"name"#,
    ];

    for invalid_username in invalid_usernames {
        let payload = serde_json::json!({
            "user_name": invalid_username,
            "password": &app.test_user.password
        });

        let response = app.login_with(&payload).await;
        let status = response.status().as_u16();

        assert_eq!(
            401, status,
            "Expected 401 for username with forbidden characters: {invalid_username}"
        );
    }
}

#[tokio::test]
async fn login_does_not_leak_validation_vs_auth_failure() {
    let app = helpers::spawn_app().await;

    // Validation failure (empty username)
    let validation_payload = serde_json::json!({
        "user_name": "",
        "password": &app.test_user.password
    });

    let validation_response = app.login_with(&validation_payload).await;
    let validation_body = validation_response.text().await.unwrap();

    // Auth failure (wrong password)
    let auth_payload = serde_json::json!({
        "user_name": &app.test_user.user_name,
        "password": "wrong_password_123"
    });

    let auth_response = app.login_with(&auth_payload).await;
    let auth_body = auth_response.text().await.unwrap();

    // Both should return 401 with generic message
    // Should not distinguish between validation failure and authentication failure
    assert!(
        !validation_body.contains("validation"),
        "Response leaked that failure was due to validation: {validation_body}"
    );
    assert!(
        !validation_body.contains("empty"),
        "Response leaked specific validation details: {validation_body}"
    );
    assert!(
        !auth_body.contains("password"),
        "Response leaked that password was wrong: {auth_body}"
    );
}

#[tokio::test]
async fn login_rejects_wrong_field_names() {
    let app = helpers::spawn_app().await;

    // Using "username" instead of "user_name"
    let payload = serde_json::json!({
        "username": &app.test_user.user_name,
        "password": &app.test_user.password
    });

    let response = app.login_with(&payload).await;

    assert!(
        response.status().as_u16() == 400 || response.status().as_u16() == 422,
        "Expected 400 or 422 for incorrect field names in JSON"
    );
}
