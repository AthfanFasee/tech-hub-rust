use super::{NewsletterHtml, NewsletterText};

#[derive(Debug)]
pub struct NewsletterContent {
    pub html: NewsletterHtml,
    pub text: NewsletterText,
}

impl NewsletterContent {
    pub fn new(html: String, text: String) -> Result<Self, String> {
        Ok(Self {
            html: NewsletterHtml::parse(html)?,
            text: NewsletterText::parse(text)?,
        })
    }
}
