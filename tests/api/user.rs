use crate::helpers::spawn_app;

#[tokio::test]
async fn add_user_returns_a_200_for_valid_json_data() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    let payload = serde_json::json!({
        "name": "athfantest",
        "email": "athfantest@gmail.com"
    });

    let response = client
        .post(format!("{}/user/add", app.address))
        .header("Content-Type", "application/json")
        .json(&payload) // Automatically sets body and content-type
        .send()
        .await
        .expect("failed to execute request");

    assert!(response.status().is_success());

    let saved = sqlx::query!("SELECT email, name FROM users",)
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved user data.");

    assert_eq!(saved.email, "athfantest@gmail.com");
    assert_eq!(saved.name, "athfantest");
}

#[tokio::test]
async fn add_user_returns_a_400_when_data_is_missing() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    let test_cases = vec![
        (serde_json::json!({ "name": "athfan" }), "missing the email"),
        (serde_json::json!({ "email": "athfantest@gmail.com" }), "missing the name"),
        (serde_json::json!({}), "missing both name and email"),
    ];

    for (invalid_payload, _error_message) in test_cases {
        let response = client
            .post(format!("{}/user/add", &app.address))
            .header("Content-Type", "application/json")
            .json(&invalid_payload)
            .send()
            .await
            .expect("Failed to execute request.");

        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request when the payload was {_error_message}."
        );
    }
}

#[tokio::test]
async fn add_user_returns_a_400_when_data_is_present_but_invalid() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    let test_cases = vec![
        (serde_json::json!({ "name": "athfan", "email": "" }), "empty email string"),
        (serde_json::json!({ "email": "athfantest@gmail.com", "name": "" }), "empty name string"),
        (serde_json::json!({"name": "athfan", "email": "definitely wrong email"}), "invalid email address"),
        (serde_json::json!({"name": "ath/fan)", "email": "athfantest@gmail.com"}), "name contains invalid characters"),
    ];

    for (invalid_payload, _error_message) in test_cases {
        let response = client
            .post(format!("{}/user/add", &app.address))
            .header("Content-Type", "application/json")
            .json(&invalid_payload)
            .send()
            .await
            .expect("Failed to execute request.");

        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request when the payload was {_error_message}."
        );
    }
}