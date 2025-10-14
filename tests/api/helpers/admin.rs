use crate::helpers::TestApp;
use reqwest::{Response, header::HeaderMap};
use serde_json::Value;
use techhub::newsletter_delivery_worker::{ExecutionOutcome, try_execute_task};

impl TestApp {
    pub async fn login_admin(&self) {
        let body = serde_json::json!({
            "user_name": "athfan",
            "password": "athfan123",
        });

        let response = self.send_post("v1/user/login", &body).await;
        assert_eq!(response.status().as_u16(), 200);
    }

    pub async fn publish_newsletters(
        &self,
        payload: &Value,
        idempotency_key: Option<&String>,
    ) -> Response {
        if let Some(key) = idempotency_key {
            let mut headers = HeaderMap::new();
            headers.insert("Idempotency-Key", key.parse().unwrap());
            self.send_post_with_headers("v1/admin/me/newsletters/publish", payload, &headers)
                .await
        } else {
            self.send_post("v1/admin/me/newsletters/publish", payload)
                .await
        }
    }

    pub async fn dispatch_all_pending_newsletter_emails(&self) {
        loop {
            if let ExecutionOutcome::EmptyQueue =
                try_execute_task(&self.db_pool, &self.email_client)
                    .await
                    .unwrap()
            {
                break;
            }
        }
    }

    pub async fn send_subscribe_email(&self) -> Response {
        self.send_get("v1/user/me/email/subscribe").await
    }
}
