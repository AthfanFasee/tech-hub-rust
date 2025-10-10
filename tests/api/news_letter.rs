use crate::helpers::{ConfirmationLinks, TestApp, TestUser, spawn_app};
use wiremock::matchers::{any, method, path};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn newsletters_are_not_delivered_to_inactivated_user() {
    let app = spawn_app().await;
    create_inactivated_user(&app).await;

    app.login_admin().await;

    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&app.email_server)
        .await;

    let body = serde_json::json!({
        "title": "Newsletter title",
        "content": {
            "text": "Plain text",
            "html": "<p>HTML</p>"
        }
    });

    let response = app.publish_newsletters(body).await;
    assert_eq!(response.status().as_u16(), 200);
}

#[tokio::test]
async fn newsletters_are_not_delivered_to_confirmed_but_unsubscribed_users() {
    let app = spawn_app().await;
    create_activated_user(&app).await;

    app.login_admin().await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&app.email_server)
        .await;

    let body = serde_json::json!({
        "title": "Newsletter title",
        "content": {
            "text": "Plain text",
            "html": "<p>HTML</p>"
        }
    });

    let response = app.publish_newsletters(body).await;
    assert_eq!(response.status().as_u16(), 200);
}

#[tokio::test]
async fn newsletters_are_delivered_to_a_user_who_subscribed_via_the_full_flow() {
    let app = spawn_app().await;

    // Log in as test user who is not subscribed by default
    let login_body = serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password
    });
    app.login(&login_body).await;

    // Scoped block ensures the first mock is dropped early
    {
        // Mock email sending for the subscription step
        let _subscription_mock = Mock::given(path("/email"))
            .and(method("POST"))
            .respond_with(ResponseTemplate::new(200))
            .named("Subscription confirmation email")
            .expect(1)
            .mount_as_scoped(&app.email_server)
            .await;

        // Send the subscribe email
        app.send_subscribe_email().await;
        
        // stimulate that user won't be clicking confirmation email in our app by logging out
        app.logout().await;

        let email_request = &app.email_server.received_requests().await.unwrap()[0];
        let confirmation_links = app.get_confirmation_links(email_request);
        reqwest::get(confirmation_links.html)
            .await
            .unwrap()
            .error_for_status()
            .unwrap();
    }
    // _subscription_mock guard dropped here

    // Log in as admin
    app.login_admin().await;

    // Scoped mock for newsletter delivery
    let _newsletter_mock = Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .named("Newsletter delivery")
        .expect(1)
        .mount_as_scoped(&app.email_server)
        .await;

    // Publish newsletter
    let newsletter_body = serde_json::json!({
        "title": "Test Newsletter",
        "content": {
            "text": "Hello subscribers!",
            "html": "<p>Hello subscribers!</p>"
        }
    });

    let response = app.publish_newsletters(newsletter_body).await;
    assert_eq!(response.status().as_u16(), 200);
}

#[tokio::test]
async fn newsletters_returns_400_for_invalid_data() {
    let app = spawn_app().await;
    app.login_admin().await;

    let invalid_cases = vec![
        (
            serde_json::json!({ "content": { "text": "Body", "html": "<p>HTML</p>" } }),
            "missing title",
        ),
        (
            serde_json::json!({ "title": "Newsletter!" }),
            "missing content",
        ),
    ];

    for (invalid_body, desc) in invalid_cases {
        let response = app.publish_newsletters(invalid_body).await;
        assert_eq!(
            400,
            response.status().as_u16(),
            "Did not return 400 when payload was {desc}"
        );
    }
}

#[tokio::test]
async fn non_admins_are_rejected_to_publish_newsletters() {
    let app = spawn_app().await;
    app.login(&serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password
    }))
    .await;

    let body = serde_json::json!({
        "title": "Newsletter title",
        "content": {
            "text": "Plain text",
            "html": "<p>HTML</p>"
        }
    });

    let response = app.publish_newsletters(body).await;
    assert_eq!(response.status().as_u16(), 403);
}

#[tokio::test]
async fn anonymous_users_cannot_publish_newsletters() {
    let app = spawn_app().await;
    let body = serde_json::json!({
        "title": "Unauthorized attempt",
        "content": {
            "text": "Plain text",
            "html": "<p>HTML</p>"
        }
    });

    let response = app.publish_newsletters(body).await;
    assert_eq!(response.status().as_u16(), 401);
}

async fn create_inactivated_user(app: &TestApp) -> ConfirmationLinks {
    let user = TestUser::generate();
    let payload = serde_json::json!({
        "name": user.username,
        "email": user.email
    });

    let _mock_guard = Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .named("Create inactivated user")
        .expect(1)
        .mount_as_scoped(&app.email_server)
        .await;

    app.register_user(&payload)
        .await
        .error_for_status()
        .unwrap();

    let email_request = &app
        .email_server
        .received_requests()
        .await
        .unwrap()
        .pop()
        .unwrap();

    app.get_confirmation_links(email_request)
}

async fn create_activated_user(app: &TestApp) {
    // Reuse the same helper and just add an extra step to actually call the confirmation link!
    let confirmation_link = create_inactivated_user(app).await;
    reqwest::get(confirmation_link.html)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
}
