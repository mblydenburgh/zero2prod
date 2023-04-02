use actix_web::{web, HttpResponse, ResponseError };
use actix_web::http::StatusCode;
use chrono::Utc;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use sqlx::{PgPool, Postgres, Transaction};
use tracing::info;
use uuid::Uuid;

use crate::{
    domain::{NewSubscriber, SubscriberEmail, SubscriberName},
    email_client::EmailClient,
    startup::ApplicationBaseUrl,
};

#[derive(serde::Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

// the impl of TryFrom is a built in way to peform the following manual failable conversion
//pub fn parse_subscriber(form: FormData) -> Result<NewSubscriber, String> {
//    let name = SubscriberName::parse(form.name)?;
//    let email = SubscriberEmail::parse(form.email)?;
//    Ok(NewSubscriber { name, email })
//}
impl TryFrom<FormData> for NewSubscriber {
    type Error = String; // type of the error
    fn try_from(value: FormData) -> Result<Self, Self::Error> {
        let name = SubscriberName::parse(value.name)?;
        let email = SubscriberEmail::parse(value.email)?;
        Ok(NewSubscriber { email, name })
    }
}


#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(form, connection_pool, base_url),
    fields(
        subscriber_email = %form.email,
        subscriber_name = %form.name
    )
)]
pub async fn subscribe(
    form: web::Form<FormData>,
    connection_pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    base_url: web::Data<ApplicationBaseUrl>,
) -> Result<HttpResponse, SubscribeError> {
    // alternate way to parse would be form.0.try_into(), since any type that
    // implements TryFrom gets an impl TryInto for free
    let new_subscriber = NewSubscriber::try_from(form.0)?;
    // Cnstruct new DB Transaction instance to pass into db method instead of the pool itself
    let mut transaction = connection_pool.begin().await.map_err(SubscribeError::PoolError)?;
    // Save subscriber to db with pending_confirm status
    let subscriber_id = save_subscriber(&new_subscriber, &mut transaction).await.map_err(SubscribeError::InsertSubscriberError)?;
    // Generate and save token to send back in confirm email
    let subscribe_token = generate_subscribe_token();
    save_token(subscriber_id, &subscribe_token, &mut transaction).await?;
    transaction.commit().await.map_err(SubscribeError::TransactionCommitError)?;
    send_confirmation_email(&email_client, new_subscriber, &base_url.0, &subscribe_token).await?;
    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument(
    name = "Send confirmation email to new subscriber",
    skip(email_client, subscriber)
)]
pub async fn send_confirmation_email(
    email_client: &EmailClient,
    subscriber: NewSubscriber,
    base_url: &str,
    subscribe_token: &str,
) -> Result<(), reqwest::Error> {
    let confirmation_link =
        format!("{base_url}/subscriptions/confirm?subscription_token={subscribe_token}");
    let subject = "Welcome!";
    let html_content = &format!(
        "Welcome to my newsletter! <br>\
        Click <a href=\"{confirmation_link}\">here</a> to confirm your subscription."
    );
    let text_content = &format!(
        "Welcome to my newsletter!\nVisit {confirmation_link} to confirm your subscription."
    );
    email_client
        .send_email(subscriber.email, subject, html_content, text_content)
        .await
}

#[tracing::instrument(
    name = "Saving subscriber to databse",
    skip(new_subscriber, transaction)
)]
pub async fn save_subscriber(
    new_subscriber: &NewSubscriber,
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<Uuid, sqlx::Error> {
    let subscriber_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at, status)
        VALUES ($1, $2, $3, $4, 'pending_confirmation')
        "#,
        subscriber_id,
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now()
    )
    .execute(transaction)
    .await
    .map_err(|err| {
        tracing::error!("Failed to execute insert: {:?}", err);
        err
    })?;
    Ok(subscriber_id)
}

#[tracing::instrument(
    name = "Saving subscriber token"
    skip(subscribe_token, transaction)
)]
pub async fn save_token(
    subscriber_id: Uuid,
    subscribe_token: &str,
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<(), StoreTokenError> {
    sqlx::query!(
        r#"
        INSERT INTO subscription_tokens (subscription_token, subscriber_id)
        VALUES ($1, $2)
        "#,
        subscribe_token,
        subscriber_id
    )
    .execute(transaction)
    .await
    .map_err(|err| {
        tracing::error!("Failed to execute insert: {:?}", err);
        StoreTokenError(err)
    })?;
    Ok(())
}

// Generates a random 25-character case-sensitive token
fn generate_subscribe_token() -> String {
    let mut rng = thread_rng();
    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
}

// Store all the types of ways a SubscribeError can happen, impl From trait for each
pub enum SubscribeError {
    ValidationError(String),
    PoolError(sqlx::Error),
    InsertSubscriberError(sqlx::Error),
    TransactionCommitError(sqlx::Error),
    StoreTokenError(StoreTokenError),
    SendEmailError(reqwest::Error)
}
impl From<reqwest::Error> for SubscribeError {
    fn from(value: reqwest::Error) -> Self {
        Self::SendEmailError(value)
    }
}
// impl From<sqlx::Error> for SubscribeError {
//     fn from(value: sqlx::Error) -> Self {
//         Self::DatabaseError(value)
//     }
// }
impl From<StoreTokenError> for SubscribeError {
    fn from(value: StoreTokenError) -> Self {
        Self::StoreTokenError(value)
    }
}
impl From<String> for SubscribeError {
    fn from(value: String) -> Self {
        Self::ValidationError(value)
    }
}

impl std::fmt::Display for SubscribeError{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
           SubscribeError::ValidationError(err) => write!(f, "{}", err),
           SubscribeError::PoolError(_) => write!(
               f, "Failed to acquire connection from pool"
            ),
            SubscribeError::InsertSubscriberError(_) => write!(
                f, "Failed to insert new subscriber into subscriber table"
            ),
            SubscribeError::TransactionCommitError(_) => write!(
                f, "Failed to commit transaction to store new subscrber"
            ),
           SubscribeError::StoreTokenError(_) => write!(
               f, "Failed to store confirmation token for new subscriber"
            ),
            SubscribeError::SendEmailError(_) => write!(
                f, "Failed to send confimation email to new subscriber"
            )
        }
    }
}
impl std::fmt::Debug for SubscribeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}
impl std::error::Error for SubscribeError{
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            // &str doesnt implement Error, so we consider it the root cause
            SubscribeError::ValidationError(_) => None,
            SubscribeError::PoolError(err) => Some(err),
            SubscribeError::InsertSubscriberError(err) => Some(err),
            SubscribeError::TransactionCommitError(err) => Some(err),
            SubscribeError::StoreTokenError(err) => Some(err),
            SubscribeError::SendEmailError(err) => Some(err)
        }
    }
}
impl ResponseError for SubscribeError {
    fn status_code(&self) -> StatusCode {
       match self {
           SubscribeError::ValidationError(_) => StatusCode::BAD_REQUEST,
           SubscribeError::TransactionCommitError(_)
               | SubscribeError::InsertSubscriberError(_)
               | SubscribeError::PoolError(_)
               | SubscribeError::StoreTokenError(_)
               | SubscribeError::SendEmailError(_) => StatusCode::INTERNAL_SERVER_ERROR
       }
    }
}

pub struct StoreTokenError(sqlx::Error);
impl std::fmt::Display for StoreTokenError{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,"A database error was encountered while trying to store a token")
    }
}
impl std::error::Error for StoreTokenError{
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
       Some(&self.0) 
    }
}
impl std::fmt::Debug for StoreTokenError{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}
// Iterate over error chain and print in a standard way
fn error_chain_fmt(
    err: &impl std::error::Error,
    f: &mut std::fmt::Formatter<'_>
) -> std::fmt::Result {
    write!(f, "{}\n", err)?;
    let mut current = err.source();
    while let Some(cause) = current {
        writeln!(f, "Caused by: {}", cause)?;
        current = cause.source();
    }
    Ok(())
}
