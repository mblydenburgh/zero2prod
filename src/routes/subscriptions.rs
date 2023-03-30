use actix_web::{web, HttpResponse};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::{NewSubscriber, SubscriberEmail, SubscriberName};

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
    skip(form, connection_pool),
    fields(
        subscriber_email = %form.email,
        subscriber_name = %form.name
    )
)]
pub async fn subscribe(
    form: web::Form<FormData>,
    connection_pool: web::Data<PgPool>,
) -> HttpResponse {
    // alternate way to parse would be form.0.try_into(), since any type that
    // implements TryFrom gets an imple TryInto for free
    let new_subscriber = match NewSubscriber::try_from(form.0) {
        Ok(result) => result,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };
    match save_subscriber(&new_subscriber, &connection_pool).await {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(err) => {
            tracing::error!("Failed to execute insert: {:?}", err);
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[tracing::instrument(
    name = "Saving subscriber to databse",
    skip(new_subscriber, connection_pool)
)]
pub async fn save_subscriber(
    new_subscriber: &NewSubscriber,
    connection_pool: &PgPool,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at, status)
        VALUES ($1, $2, $3, $4, 'confirmed')
        "#,
        Uuid::new_v4(),
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now()
    )
    .execute(connection_pool)
    .await
    .map_err(|err| {
        tracing::error!("Failed to execute insert: {:?}", err);
        err
    })?;
    Ok(())
}
