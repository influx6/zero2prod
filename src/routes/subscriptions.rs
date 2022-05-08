use actix_web::{web, HttpResponse, Responder};
use chrono::Utc;
use sqlx::PgPool;
use tracing;
use unicode_segmentation::UnicodeSegmentation;
use uuid::Uuid;

use crate::domain::{NewSubscriber, SubscriberName};

pub fn is_valid_name(s: &str) -> bool {
    let is_empty_or_whitespace = s.trim().is_empty();
    let is_too_long = s.graphemes(true).count() > 256;
    let forbidden_characters = ['/', '(', ')', '<', '>', '\\', '{', '}'];
    let contains_forbidden_characters = s.chars().any(|g| forbidden_characters.contains(&g));
    return !(is_empty_or_whitespace || is_too_long || contains_forbidden_characters);
}

#[derive(serde::Deserialize)]
pub struct SubscriptionForm {
    name: String,
    email: String,
}

#[tracing::instrument(
name = "Adding a new subscriber",
skip(form, pool),
fields(
subscriber_email = % form.email,
subscriber_name = % form.name,
)
)]
pub async fn subscribe(
    form: web::Form<SubscriptionForm>,
    pool: web::Data<PgPool>,
) -> impl Responder {
    let name = match SubscriberName::parse(form.name.clone()) {
        Ok(name) => name,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };

    let new_subscriber = NewSubscriber {
        email: form.email.clone(),
        name,
    };

    return match insert_subscriber(&pool, &new_subscriber).await {
        Ok(_) => {
            tracing::info!(
                "Created new subscription for user: {} and email: {} in db",
                form.name,
                form.email,
            );
            HttpResponse::Ok().finish()
        }
        Err(_) => HttpResponse::InternalServerError().finish(),
    };
}

#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(subscriber, pool)
)]
pub async fn insert_subscriber(
    pool: &PgPool,
    subscriber: &NewSubscriber,
) -> Result<(), sqlx::Error> {
    // insert record into database.
    sqlx::query!(
        r#"
    INSERT INTO subscriptions (id, email, name, subscribed_at)
    VALUES ($1,$2,$3,$4)
    "#,
        Uuid::new_v4(),
        subscriber.email,
        subscriber.name.as_ref(),
        Utc::now()
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
        // ? will cause the error to be handled. Think of it like
        // say raise/throw exception in python/java.
    })?;

    Ok(())
}
