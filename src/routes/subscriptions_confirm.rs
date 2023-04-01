use actix_web::{web, HttpResponse};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

#[tracing::instrument(name = "Confirm a pending subscriber", skip(parameters))]
pub async fn confirm(
    parameters: web::Query<Parameters>,
    connection_pool: web::Data<PgPool>,
) -> HttpResponse {
    // Get token from params
    let token = &parameters.subscription_token;
    // Get subscriber id from provided token
    let subscriber_id = match get_subscriber_id_from_token(token, &connection_pool).await {
        Ok(id) => id,
        Err(_) => {
            return HttpResponse::InternalServerError().finish();
        }
    };
    // Validate returned id has a value
    let id = match subscriber_id {
        Some(val) => val,
        None => {
            return HttpResponse::Unauthorized().finish();
        }
    };
    // Update subscriber status with found id
    if update_subscriber_status(&id, &connection_pool)
        .await
        .is_err()
    {
        return HttpResponse::InternalServerError().finish();
    }
    HttpResponse::Ok().finish()
}

#[tracing::instrument(name = "Getting subscriber id from token", skip(token))]
pub async fn get_subscriber_id_from_token(
    token: &str,
    connection_pool: &PgPool,
) -> Result<Option<Uuid>, sqlx::Error> {
    let result = sqlx::query!(
        "SELECT subscriber_id FROM subscription_tokens WHERE subscription_token = $1",
        token
    )
    .fetch_optional(connection_pool)
    .await
    .map_err(|err| {
        tracing::error!("Could not execute query: {:?}", err);
        err
    })?;
    // result is a db row, with the selected fields as properties to access of the result struct
    Ok(result.map(|res| res.subscriber_id))
}

#[tracing::instrument(name = "Updating subscriber status", skip(subscriber_id))]
pub async fn update_subscriber_status(
    subscriber_id: &Uuid,
    connection_pool: &PgPool,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        UPDATE subscriptions
        SET status = 'confirmed'
        WHERE id = $1
        "#,
        subscriber_id,
    )
    .execute(connection_pool)
    .await
    .map_err(|err| {
        tracing::error!("Failed to execute query: {:?}", err);
        err
    })?;
    Ok(())
}
