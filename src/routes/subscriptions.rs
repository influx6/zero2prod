use actix_web::{web, HttpResponse, Responder};
use chrono::Utc;
use sqlx::PgPool;
use tracing;
use tracing::Instrument;
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct SubscriptionForm {
    name: String,
    email: String,
}

pub async fn subscribe(
    form: web::Form<SubscriptionForm>,
    pool: web::Data<PgPool>,
) -> impl Responder {
    let request_id = Uuid::new_v4();
    let request_span = tracing::info_span!(
        "Adding a new subscriber",
        %request_id,
        subscriber_email = %form.email,
        subscriber_name = %form.name,
    );

    let _request_span_guard = request_span.enter();

    tracing::info!(
        "Received subscription request for name: {} with email: {}",
        form.name.clone(),
        form.email.clone()
    );

    let query_span = tracing::info_span!("Saving new subscriber details into the database");

    // insert record into database.
    match sqlx::query!(
        r#"
    INSERT INTO subscriptions (id, email, name, subscribed_at)
    VALUES ($1,$2,$3,$4)
    "#,
        Uuid::new_v4(),
        form.email,
        form.name,
        Utc::now()
    )
    .execute(pool.get_ref())
    .instrument(query_span)
    .await
    {
        Ok(_) => {
            tracing::info!(
                "Created new subscription for user: {} and email: {} in db",
                form.name.clone(),
                form.email.clone()
            );
            HttpResponse::Ok()
        }
        Err(e) => {
            tracing::info!(
                "Failed to create db records for subscription for user: {} and email: {} in db, error: {:?}",
                form.name.clone(),
                form.email.clone(),
                e,
            );
            HttpResponse::InternalServerError()
        }
    }
}
