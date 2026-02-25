use reqwest::Response;
use serde_json::Value;
use uuid::Uuid;

use crate::helpers::TestApp;

impl TestApp {
    pub async fn create_post(&self, payload: &Value) -> Response {
        self.send_post("v1/posts/me/create", payload).await
    }

    pub async fn update_post(&self, id: &Uuid, payload: &Value) -> Response {
        self.send_patch_with_payload(&format!("v1/posts/me/update/{id}"), payload)
            .await
    }

    pub async fn delete_post(&self, id: &Uuid) -> Response {
        self.send_delete(&format!("v1/posts/me/delete/{id}")).await
    }

    pub async fn hard_delete_post(&self, id: &Uuid) -> Response {
        self.send_delete(&format!("v1/admin/me/posts/delete/{id}"))
            .await
    }

    pub async fn like_post(&self, id: &Uuid) -> Response {
        self.send_patch(&format!("v1/posts/me/like/{id}")).await
    }

    pub async fn dislike_post(&self, id: &Uuid) -> Response {
        self.send_patch(&format!("v1/posts/me/dislike/{id}")).await
    }

    pub async fn get_post(&self, id: &Uuid) -> Response {
        self.send_get(&format!("v1/posts/get/{id}")).await
    }

    pub async fn get_all_posts(&self, query: &str) -> Response {
        self.send_get(&format!("v1/posts/get/all{query}")).await
    }
}
