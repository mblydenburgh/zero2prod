use reqwest::Client;
use secrecy::{Secret, ExposeSecret};

use crate::domain::SubscriberEmail;

pub struct EmailClient {
    base_url: reqwest::Url,
    http_client: Client,
    sender: SubscriberEmail,
    token: Secret<String>
}

impl EmailClient {
    pub fn new(base_url: String, sender: SubscriberEmail, token: Secret<String>) -> Self {
        let http_client = Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap();
        Self {
            http_client,
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
            from: self.sender.as_ref(),
            to: recipient.as_ref(),
            subject: &subject,
            html_body: html_content,
            text_body: text_content
        };
        let _builder = self.http_client
            .post(url.as_str())
            .json(&request_body)
            .header("X-Postmark-Server-Token", self.token.expose_secret())
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }
}

// Since we are trying to store a reference to a place in memory, we need to ensure
// the compiler knows how long these references are valid for.
#[derive(serde::Serialize)]
#[serde(rename_all = "PascalCase")]
struct SendEmailRequest<'a> {
    from: &'a str,
    to: &'a str,
    subject: &'a str,
    html_body: &'a str,
    text_body: &'a str 
}

#[cfg(test)]
mod tests {
    use crate::domain::SubscriberEmail;
    use crate::email_client::EmailClient;
    use claims::{assert_ok, assert_err};
    use fake::faker::internet::en::SafeEmail;
    use fake::faker::lorem::en::{Paragraph, Sentence};
    use fake::{Fake, Faker};
    use secrecy::Secret;
    use wiremock::Request;
    use wiremock::matchers::{header_exists, header, path, method, any};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    struct SendEmailBodyMatcher;

    impl wiremock::Match for SendEmailBodyMatcher {
        fn matches(&self, request: &Request) -> bool {
            let result: Result<serde_json::Value, _> = serde_json::from_slice(&request.body);
            if let Ok(body) = result {
                // Check if keys are present, ignoring values
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
    async fn send_email_sends_the_expected_requessend_email_fires_a_request_to_base_urlt() {
        // Arrange
        let mock_server = MockServer::start().await;
        let sender = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        let email_client = EmailClient::new(
            mock_server.uri(),
            sender,
            Secret::new(Faker.fake())
        );
        Mock::given(header_exists("X-Postmark-Server-Token"))
            .and(header("Content-Type", "application/json"))
            .and(path("/email"))
            .and(method("POST"))
            .and(SendEmailBodyMatcher)
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

    #[tokio::test]
    async fn send_email_succeeds_if_server_returns_a_200() {
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
        let outcome = email_client.send_email(subscriber_email, &subject, &body, &body)
            .await;
        // Assert
        assert_ok!(outcome);
    }

    #[tokio::test]
    async fn send_email_fails_if_server_returns_a_500() {
        // Arrange
        let mock_server = MockServer::start().await;
        let sender = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        let email_client = EmailClient::new(
            mock_server.uri(),
            sender,
            Secret::new(Faker.fake())
        );
        Mock::given(any())
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&mock_server)
            .await;
        let subscriber_email = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        let subject: String = Sentence(1..2).fake();
        let body: String = Paragraph(1..10).fake();
        // Act
        let outcome = email_client.send_email(subscriber_email, &subject, &body, &body)
            .await;
        // Assert
        assert_err!(outcome);
    }

    #[tokio::test]
    async fn send_email_times_out_if_server_takes_too_long() {
        // Arrange
        let mock_server = MockServer::start().await;
        let sender = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        let email_client = EmailClient::new(
            mock_server.uri(),
            sender,
            Secret::new(Faker.fake())
        );
        let response = ResponseTemplate::new(200)
            .set_delay(std::time::Duration::from_secs(180));
        Mock::given(any())
            .respond_with(response)
            .expect(1)
            .mount(&mock_server)
            .await;
        let subscriber_email = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        let subject: String = Sentence(1..2).fake();
        let body: String = Paragraph(1..10).fake();
        // Act
        let outcome = email_client.send_email(subscriber_email, &subject, &body, &body)
            .await;
        // Assert
        assert_err!(outcome);
    }
}
