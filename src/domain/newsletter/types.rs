use serde::Deserialize;

use crate::domain::Newsletter;

#[derive(Deserialize, Debug)]
pub struct NewsLetterContentPayload {
    html: String,
    text: String,
}

#[derive(Deserialize, Debug)]
pub struct NewsLetterData {
    title: String,
    content: NewsLetterContentPayload,
}

impl TryFrom<NewsLetterData> for Newsletter {
    type Error = String;

    fn try_from(payload: NewsLetterData) -> Result<Self, Self::Error> {
        Newsletter::new(payload.title, payload.content.html, payload.content.text)
    }
}

pub struct NewsletterIssue {
    title: String,
    text_content: String,
    html_content: String,
}

impl NewsletterIssue {
    pub fn new(title: String, text_content: String, html_content: String) -> Self {
        Self {
            title,
            text_content,
            html_content,
        }
    }
    pub fn title(&self) -> &str {
        &self.title
    }
    pub fn text_content(&self) -> &str {
        &self.text_content
    }
    pub fn html_content(&self) -> &str {
        &self.html_content
    }
}
