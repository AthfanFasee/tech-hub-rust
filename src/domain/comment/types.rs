use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::Comment;

#[derive(sqlx::FromRow)]
pub struct CommentRecord {
    pub id: Uuid,
    pub text: String,
    pub post_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub created_by: Uuid,
    pub user_name: String,
}

// For creating comments - borrows data
#[derive(Serialize, Debug)]
pub struct CreateCommentResponseBody<'a> {
    pub id: Uuid,
    pub text: &'a str,
    pub post_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub created_by: Uuid,
}

// For fetching comments - owns data
#[derive(Serialize, Debug)]
pub struct CommentResponseBody {
    pub id: Uuid,
    pub text: String,
    pub post_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub created_by: Uuid,
}

impl From<CommentRecord> for CommentResponseBody {
    fn from(record: CommentRecord) -> Self {
        Self {
            id: record.id,
            text: record.text,
            post_id: record.post_id,
            created_at: record.created_at,
            created_by: record.created_by,
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct CreateCommentPayload {
    pub text: String,
    pub post_id: String,
}

impl TryFrom<CreateCommentPayload> for Comment {
    type Error = String;

    fn try_from(value: CreateCommentPayload) -> Result<Self, Self::Error> {
        Comment::new(value.text, value.post_id)
    }
}
