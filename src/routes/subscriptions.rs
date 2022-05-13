use actix_web::{web, HttpResponse, Responder};
use chrono::Utc;
use sqlx::PgPool;
use tracing;
use uuid::Uuid;

use crate::domain::new_subscriber::NewSubscriber;
use crate::domain::subscriber_email::SubscriberEmail;
use crate::domain::subscriber_name::SubscriberName;
use crate::mail::send_email::EmailClient;

#[derive(serde::Deserialize)]
pub struct SubscriptionForm {
    name: String,
    email: String,
}

impl TryFrom<SubscriptionForm> for NewSubscriber {
    type Error = String;

    fn try_from(form: SubscriptionForm) -> Result<Self, Self::Error> {
        let name = SubscriberName::parse(form.name)?;
        let email = SubscriberEmail::parse(form.email)?;
        Ok(Self { email, name })
    }
}

#[tracing::instrument(
name = "Adding a new subscriber",
skip(form, pool, email_client),
fields(
subscriber_email = % form.email,
subscriber_name = % form.name,
)
)]
pub async fn subscribe(
    form: web::Form<SubscriptionForm>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
) -> impl Responder {
    let new_subscriber = match form.0.try_into() {
        Ok(name) => name,
        Err(_) => return HttpResponse::BadRequest(),
    };

    if insert_subscriber(&pool, &new_subscriber).await.is_err() {
        return HttpResponse::InternalServerError();
    };

    if send_confirmation_email(&email_client, new_subscriber)
        .await
        .is_err()
    {
        tracing::error!("Failed to send email request");
        return HttpResponse::InternalServerError();
    }

    HttpResponse::Ok()
}

#[tracing::instrument(
    name = "Send confirmation email to a new subscriber",
    skip(email_client, new_subscriber)
)]
pub async fn send_confirmation_email(
    email_client: &EmailClient,
    new_subscriber: NewSubscriber,
) -> Result<(), reqwest::Error> {
    let confirmation_link = "htts://my-api.com/subscriptions/confirm";
    email_client
        .send_email(
            new_subscriber.email,
            "Welcome!",
            &format!(
                "Welcome to our newsletter!<br />\
                Click <a href=\"{}\">here</a> to confirm your subscription.
                ",
                confirmation_link,
            ),
            &format!(
                "Welcome to our newsletter!\n Visit {} to confirm your subscriptions.",
                confirmation_link,
            ),
        )
        .await
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
    INSERT INTO subscriptions (id, email, name, subscribed_at, status)
    VALUES ($1,$2,$3,$4,'pending_confirmation')
    "#,
        Uuid::new_v4(),
        subscriber.email.as_ref(),
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
