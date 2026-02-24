use crate::domain::UserEmail;
use reqwest::{Client, Url};
use secrecy::{ExposeSecret, Secret};
use std::time::Duration;

#[derive(Debug)]
pub struct EmailClient {
    http_client: Client,
    base_url: Url,
    sender: UserEmail,
    authorization_token: Secret<String>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "PascalCase")]
struct SendEmailRequest<'a> {
    from: &'a str,
    to: &'a str,
    subject: &'a str,
    html_body: &'a str,
    text_body: &'a str,
}

#[derive(thiserror::Error, Debug)]
pub enum EmailError {
    #[error(transparent)]
    Request(#[from] reqwest::Error),

    #[error(transparent)]
    Url(#[from] url::ParseError),
}

impl EmailClient {
    pub fn new(
        base_url: Url,
        sender: UserEmail,
        authorization_token: Secret<String>,
        timeout: Duration,
    ) -> Self {
        let http_client = Client::builder().timeout(timeout).build().unwrap();

        Self {
            http_client,
            base_url,
            sender,
            authorization_token,
        }
    }
    pub async fn send_email(
        &self,
        recipient: &UserEmail,
        subject: &str,
        html_content: &str,
        text_content: &str,
    ) -> Result<(), EmailError> {
        let url = self.base_url.join("/email")?;

        let request_body = SendEmailRequest {
            from: self.sender.as_ref(),
            to: recipient.as_ref(),
            subject,
            html_body: html_content,
            text_body: text_content,
        };

        self.http_client
            .post(url)
            .header(
                "X-Postmark-Server-Token",
                self.authorization_token.expose_secret(),
            )
            .json(&request_body)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::UserEmail;
    use crate::email_client::EmailClient;
    use claims::{assert_err, assert_ok};
    use fake::faker::internet;
    use fake::faker::lorem;
    use fake::{Fake, Faker};
    use reqwest::Url;
    use secrecy::Secret;
    use serde_json::Value;
    use std::time::Duration;
    use wiremock::matchers;
    use wiremock::{Match, Mock, MockServer, Request, ResponseTemplate};

    struct SendEmailBodyMatcher;

    impl Match for SendEmailBodyMatcher {
        fn matches(&self, request: &Request) -> bool {
            let result: Result<Value, _> = serde_json::from_slice(&request.body);

            if let Ok(body) = result {
                body.get("From").is_some()
                    && body.get("To").is_some()
                    && body.get("Subject").is_some()
                    && body.get("HtmlBody").is_some()
                    && body.get("TextBody").is_some()
            } else {
                false
            }
        }
    }

    #[tokio::test]
    async fn send_email_sends_the_expected_request() {
        let mock_server = MockServer::start().await;

        let email_client = email_client(mock_server.uri());

        Mock::given(matchers::header_exists("X-Postmark-Server-Token"))
            .and(matchers::header("Content-Type", "application/json"))
            .and(matchers::path("/email"))
            .and(matchers::method("POST"))
            .and(SendEmailBodyMatcher)
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let _ = email_client
            .send_email(&email(), &subject(), &content(), &content())
            .await;
    }

    #[tokio::test]
    async fn send_email_succeeds_if_the_server_returns_200() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        Mock::given(matchers::any())
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let outcome = email_client
            .send_email(&email(), &subject(), &content(), &content())
            .await;

        assert_ok!(outcome);
    }

    #[tokio::test]
    async fn send_email_fails_if_the_server_returns_500() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        Mock::given(matchers::any())
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&mock_server)
            .await;

        let outcome = email_client
            .send_email(&email(), &subject(), &content(), &content())
            .await;

        assert_err!(outcome);
    }

    #[tokio::test]
    async fn send_email_times_out_if_the_server_takes_too_long() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        let response = ResponseTemplate::new(200).set_delay(Duration::from_secs(10));

        Mock::given(matchers::any())
            .respond_with(response)
            .expect(1)
            .mount(&mock_server)
            .await;

        let outcome = email_client
            .send_email(&email(), &subject(), &content(), &content())
            .await;

        assert_err!(outcome);
    }

    // Generate a random email subject
    fn subject() -> String {
        lorem::en::Sentence(1..2).fake()
    }
    // Generate a random email content
    fn content() -> String {
        lorem::en::Paragraph(1..10).fake()
    }
    // Generate a random subscriber email
    fn email() -> UserEmail {
        UserEmail::parse(internet::en::SafeEmail().fake()).unwrap()
    }

    /// Get a test instance of `EmailClient`.
    fn email_client(base_url: String) -> EmailClient {
        EmailClient::new(
            Url::parse(&base_url).unwrap(),
            email(),
            Secret::new(Faker.fake()),
            Duration::from_millis(200),
        )
    }
}
