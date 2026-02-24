use crate::domain::Newsletter;
use serde::Deserialize;

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
