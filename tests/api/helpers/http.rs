use crate::helpers::{ConfirmationLinks, TestApp, TestUser};
use linkify::{LinkFinder, LinkKind};
use reqwest::{Response, header::HeaderMap};
use serde_json::Value;
use uuid::Uuid;
use wiremock::matchers;
use wiremock::{Mock, Request, ResponseTemplate};

impl TestApp {
    pub fn get_confirmation_links(&self, email_request: &Request) -> ConfirmationLinks {
        let body: Value = serde_json::from_slice(&email_request.body).unwrap();

        let get_link = |s: &str| {
            let links: Vec<_> = LinkFinder::new()
                .links(s)
                .filter(|l| *l.kind() == LinkKind::Url)
                .collect();
            assert_eq!(links.len(), 1);
            let mut link = reqwest::Url::parse(links[0].as_str()).unwrap();
            assert_eq!(link.host_str().unwrap(), "127.0.0.1");
            link.set_port(Some(self.port)).unwrap();
            link
        };

        let html = get_link(body["HtmlBody"].as_str().unwrap());
        let plain_text = get_link(body["TextBody"].as_str().unwrap());
        ConfirmationLinks { html, plain_text }
    }

    pub async fn create_inactivated_user(&self) -> (Value, ConfirmationLinks) {
        let user = TestUser::generate();
        let payload = serde_json::json!({
            "user_name": user.user_name,
            "email": user.email,
            "password": user.password,
        });

        let _mock_guard = Mock::given(matchers::path("/email"))
            .and(matchers::method("POST"))
            .respond_with(ResponseTemplate::new(200))
            .named("Create inactivated user")
            .expect(1)
            .mount_as_scoped(&self.email_server)
            .await;

        self.register_user(&payload)
            .await
            .error_for_status()
            .unwrap();

        let email_request = &self
            .email_server
            .received_requests()
            .await
            .unwrap()
            .pop()
            .unwrap();

        let confirmation_links = self.get_confirmation_links(email_request);
        (payload, confirmation_links)
    }

    pub async fn create_activated_user(&self) -> Value {
        let (payload, confirmation_link) = self.create_inactivated_user().await;
        reqwest::get(confirmation_link.html)
            .await
            .unwrap()
            .error_for_status()
            .unwrap();
        payload
    }

    pub async fn create_active_subscriber(&self) {
        let payload = self.create_activated_user().await;

        let response = self.login_with(&payload).await;
        assert_eq!(response.status().as_u16(), 200);

        let _mock_guard = Mock::given(matchers::path("/email"))
            .and(matchers::method("POST"))
            .respond_with(ResponseTemplate::new(200))
            .named("Subscription confirmation email")
            .expect(1)
            .mount_as_scoped(&self.email_server)
            .await;

        self.send_subscribe_email().await;

        // Stimulate that user will be clicking confirmation email outside our app by logging out
        self.logout().await;

        // Extract confirmation link from subscription email and "click" it
        let email_request = &self.email_server.received_requests().await.unwrap()[1];
        let confirmation_links = self.get_confirmation_links(email_request);
        reqwest::get(confirmation_links.html)
            .await
            .unwrap()
            .error_for_status()
            .unwrap();
    }

    pub async fn create_sample_post(&self) -> Uuid {
        let payload = serde_json::json!({
            "title": "Post for comments",
            "text": "This is a sample posts to attach comments to",
            "img": "https://example.com/posts.jpg"
        });

        let response = self.create_post(&payload).await;
        assert_eq!(response.status().as_u16(), 201);
        let body: Value = response.json().await.unwrap();
        Uuid::parse_str(body["id"].as_str().unwrap()).unwrap()
    }

    pub async fn create_sample_post_custom(&self, title: &str, text: &str) -> Uuid {
        let payload = serde_json::json!({
            "title": title,
            "text": text,
            "img": "https://example.com/sample.jpg"
        });

        let response = self.create_post(&payload).await;
        assert_eq!(
            response.status().as_u16(),
            201,
            "Failed to create sample post"
        );
        let body: Value = response.json().await.unwrap();
        Uuid::parse_str(body["id"].as_str().unwrap()).unwrap()
    }

    pub async fn like_post_as_user(&self, post_id: &Uuid) {
        let response = self.like_post(post_id).await;
        assert_eq!(response.status().as_u16(), 200, "Failed to like post");
    }

    pub async fn send_get(&self, endpoint: &str) -> Response {
        self.api_client
            .get(format!("{}/{}", self.address, endpoint))
            .send()
            .await
            .expect("GET request failed")
    }

    pub async fn send_post(&self, endpoint: &str, payload: &Value) -> Response {
        self.api_client
            .post(format!("{}/{}", self.address, endpoint))
            .json(payload)
            .send()
            .await
            .expect("POST request failed")
    }

    pub async fn send_post_with_headers(
        &self,
        endpoint: &str,
        payload: &Value,
        headers: &HeaderMap,
    ) -> Response {
        self.api_client
            .post(format!("{}/{}", self.address, endpoint))
            .json(payload)
            .headers(headers.clone())
            .send()
            .await
            .expect("POST request with headers failed")
    }

    pub async fn send_patch(&self, endpoint: &str) -> Response {
        self.api_client
            .patch(format!("{}/{}", &self.address, endpoint))
            .send()
            .await
            .expect("Failed to execute PATCH request.")
    }

    pub async fn send_patch_with_payload(&self, endpoint: &str, payload: &Value) -> Response {
        self.api_client
            .patch(format!("{}/{}", &self.address, endpoint))
            .json(payload)
            .send()
            .await
            .expect("Failed to execute PATCH request.")
    }

    pub async fn send_delete(&self, endpoint: &str) -> Response {
        self.api_client
            .delete(format!("{}/{}", &self.address, endpoint))
            .send()
            .await
            .expect("Failed to execute DELETE request.")
    }
}
