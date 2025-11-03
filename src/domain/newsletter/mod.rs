mod new_newsletter;
mod newsletter_html;
mod newsletter_text;
mod newsletter_title;

pub use new_newsletter::NewsletterContent;
pub use newsletter_html::NewsletterHtml;
pub use newsletter_text::NewsletterText;
pub use newsletter_title::NewsletterTitle;

use serde::Deserialize;

#[derive(Debug)]
pub struct Newsletter {
    pub title: NewsletterTitle,
    pub content: NewsletterContent,
}

impl Newsletter {
    pub fn new(title: String, html: String, text: String) -> Result<Self, String> {
        Ok(Self {
            title: NewsletterTitle::parse(title)?,
            content: NewsletterContent::new(html, text)?,
        })
    }
}

#[derive(Deserialize, Debug)]
pub struct NewsLetterData {
    title: String,
    content: ContentPayload,
}

#[derive(Deserialize, Debug)]
pub struct ContentPayload {
    html: String,
    text: String,
}

impl TryFrom<NewsLetterData> for Newsletter {
    type Error = String;

    fn try_from(payload: NewsLetterData) -> Result<Self, Self::Error> {
        Newsletter::new(payload.title, payload.content.html, payload.content.text)
    }
}
