use crate::helpers::{ConfirmationLinks, TestApp};
use linkify::{LinkFinder, LinkKind};
use reqwest::{Response, header::HeaderMap};
use serde_json::Value;
use wiremock::Request;

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
