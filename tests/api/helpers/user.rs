use reqwest::Response;
use serde_json::Value;

use crate::helpers::TestApp;

impl TestApp {
    pub async fn register_user(&self, payload: &Value) -> Response {
        self.send_post("v1/user/register", payload).await
    }

    pub async fn login(&self) {
        let body = serde_json::json!({
            "user_name": &self.test_user.user_name,
            "password": &self.test_user.password,
        });
        let response = self.send_post("v1/user/login", &body).await;
        assert_eq!(response.status().as_u16(), 200);
    }

    pub async fn login_with(&self, creds: &Value) -> Response {
        self.send_post("v1/user/login", creds).await
    }

    pub async fn logout(&self) -> Response {
        self.send_post("v1/user/me/logout", &serde_json::json!({}))
            .await
    }

    pub async fn change_password(&self, payload: &Value) -> Response {
        self.send_post("v1/user/me/change-password", payload).await
    }

    pub async fn request_subscription_email(&self) -> Response {
        self.send_get("v1/user/me/request-subscription").await
    }

    pub async fn access_protected(&self) -> Response {
        self.send_get("v1/user/me/protected").await
    }
}
