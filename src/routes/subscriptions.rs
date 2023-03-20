use actix_web::{web, HttpResponse};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::{NewSubscriber, SubscriberName};

#[derive(serde::Deserialize)]
pub struct FormData {
    email: String,
    name: String,
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
    let subscriber_name = SubscriberName::parse(form.name.clone());
    let new_subscriber = NewSubscriber {
        name: subscriber_name,
        email: form.0.email
    };
    match save_subscriber(&new_subscriber, &connection_pool).await {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(err) => {
            tracing::error!("Failed to execute insert: {:?}", err);
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[tracing::instrument(name = "Saving subscriber to databse", skip(new_subscriber, connection_pool))]
pub async fn save_subscriber(new_subscriber: &NewSubscriber, connection_pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at)
        VALUES ($1, $2, $3, $4)
        "#,
        Uuid::new_v4(),
        new_subscriber.email,
        new_subscriber.name.inner_ref(),
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
