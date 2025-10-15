use crate::helpers::TestApp;
use reqwest::Response;
use serde_json::Value;
use uuid::Uuid;

impl TestApp {
    pub async fn create_comment(&self, payload: &Value) -> Response {
        self.send_post("v1/comment/me/create", payload).await
    }

    pub async fn delete_comment(&self, id: &Uuid) -> Response {
        self.send_delete(&format!("v1/comment/me/delete/{id}"))
            .await
    }

    pub async fn get_comments(&self, id: &Uuid) -> Response {
        self.send_get(&format!("v1/comment/get/posts/{id}")).await
    }
}
