use crate::domain::CommentText;
use uuid::Uuid;

#[derive(Debug)]
pub struct Comment {
    pub text: CommentText,
    pub post_id: Uuid,
}

impl Comment {
    pub fn new(text: String, post_id: String) -> Result<Self, String> {
        let post_id = Uuid::parse_str(&post_id)
            .map_err(|_| "Invalid post_id: must be a valid UUID".to_string())?;

        Ok(Self {
            text: CommentText::parse(text)?,
            post_id,
        })
    }
}
