use reqwest::Client;
use secrecy::{Secret, ExposeSecret};
use tracing::info;

use crate::domain::SubscriberEmail;

pub struct EmailClient {
    base_url: reqwest::Url,
    http_client: Client,
    sender: SubscriberEmail,
    token: Secret<String>
}

impl EmailClient {
    pub fn new(base_url: String, sender: SubscriberEmail, token: Secret<String>) -> Self {
        Self {
            http_client: Client::new(),
            base_url: reqwest::Url::parse(base_url.as_str()).expect("Could not parse URL"),
            sender,
            token
        }
    }
    pub async fn send_email(
        &self,
        recipient: SubscriberEmail,
        subject: &str,
        html_content: &str,
        text_content: &str
    ) -> Result<(), reqwest::Error> {
        let url = reqwest::Url::join(&self.base_url, "/email").expect("Could not build request url");
        let request_body = SendEmailRequest {
            from: self.sender.as_ref().to_owned(),
            to: recipient.as_ref().to_owned(),
            subject: subject.to_owned(),
            html_body: html_content.to_owned(),
            text_body: text_content.to_owned()
        };
        let builder = self.http_client
            .post(url.as_str())
            .json(&request_body)
            .header("X-Postmark-Server-Token", self.token.expose_secret())
            .send()
            .await?;
        Ok(())
    }
}

#[derive(serde::Serialize)]
struct SendEmailRequest {
    from: String,
    to: String,
    subject: String,
    html_body: String,
    text_body: String
}

#[cfg(test)]
mod tests {
    use crate::domain::SubscriberEmail;
    use crate::email_client::EmailClient;
    use fake::faker::internet::en::SafeEmail;
    use fake::faker::lorem::en::{Paragraph, Sentence};
    use fake::{Fake, Faker};
    use secrecy::Secret;
    use wiremock::matchers::any;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn send_email_fires_a_request_to_base_url() {
        // Arrange
        let mock_server = MockServer::start().await;
        let sender = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        let email_client = EmailClient::new(
            mock_server.uri(),
            sender,
            Secret::new(Faker.fake())
        );
        Mock::given(any())
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;
        let subscriber_email = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        let subject: String = Sentence(1..2).fake();
        let body: String = Paragraph(1..10).fake();
        // Act
        let _ = email_client.send_email(subscriber_email, &subject, &body, &body)
            .await;
        // Assert
    }
}
