use actix_web::{web, HttpResponse};
use chrono::Utc;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use sqlx::{PgPool, Postgres, Transaction};
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
) -> HttpResponse {
    // alternate way to parse would be form.0.try_into(), since any type that
    // implements TryFrom gets an imple TryInto for free
    let new_subscriber = match NewSubscriber::try_from(form.0) {
        Ok(result) => result,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };
    // Cnstruct new DB Transaction instance to pass into db method instead of the pool itself
    let mut transaction = match connection_pool.begin().await {
        Ok(transaction) => transaction,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };
    // Save subscriber to db with pending_confirm status
    let subscriber_id = match save_subscriber(&new_subscriber, &mut transaction).await {
        Ok(id) => id,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };
    // Generate and save token to send back in confirm email
    let subscribe_token = generate_subscribe_token();
    if save_token(subscriber_id, &subscribe_token, &mut transaction)
        .await
        .is_err()
    {
        info!("error saving token");
        return HttpResponse::InternalServerError().finish();
    }
    // Send cofirmation email to subscriber
    if send_confirmation_email(&email_client, new_subscriber, &base_url.0, &subscribe_token)
        .await
        .is_err()
    {
        info!("error sending confirm email");
        return HttpResponse::InternalServerError().finish();
    }
    if transaction.commit().await.is_err() {
        info!("error commiting transaction");
        return HttpResponse::InternalServerError().finish();
    }
    HttpResponse::Ok().finish()
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
    info!("confirmation link: {}", confirmation_link);
    info!("sending to subscriber: {}", subscriber.email);
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
) -> Result<(), sqlx::Error> {
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
        err
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
