use crate::domain::Post;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(sqlx::FromRow)]
pub struct PostRecord {
    pub total_count: i64,
    pub id: Uuid,
    pub title: String,
    pub post_text: String,
    pub img: String,
    pub version: i32,
    pub liked_by: Option<Vec<Uuid>>,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub created_by_name: String,
}

#[derive(serde::Serialize)]
pub struct PostResponse {
    pub id: Uuid,
    pub title: String,
    pub text: String,
    pub img: String,
    pub version: i32,
    pub created_at: DateTime<Utc>,
    pub created_by: Uuid,
    created_by_name: String,
    #[serde(default)]
    pub liked_by: Vec<Uuid>,
}

impl From<PostRecord> for PostResponse {
    fn from(record: PostRecord) -> Self {
        Self {
            id: record.id,
            title: record.title,
            text: record.post_text,
            img: record.img,
            version: record.version,
            created_at: record.created_at,
            created_by: record.created_by,
            created_by_name: record.created_by_name,
            liked_by: record.liked_by.unwrap_or_default(),
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct CreatePostPayload {
    title: String,
    text: String,
    img: String,
}

#[derive(Serialize)]
pub struct CreatePostResponse<'a> {
    pub id: Uuid,
    pub title: &'a str,
    pub post_text: &'a str,
    pub img: &'a str,
    pub created_at: DateTime<Utc>,
    pub created_by: Uuid,
}

impl TryFrom<CreatePostPayload> for Post {
    type Error = String;

    fn try_from(payload: CreatePostPayload) -> Result<Self, Self::Error> {
        let post = Self::new(payload.title, payload.text, payload.img)?;
        Ok(post)
    }
}
