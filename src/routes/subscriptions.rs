use actix_web::{web, HttpResponse, Responder};
use chrono::Utc;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use sqlx::PgPool;
use tracing;
use uuid::Uuid;

use crate::domain::application::ApplicationBaseUrl;
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
skip(form, pool, email_client, domain),
fields(
subscriber_email = % form.email,
subscriber_name = % form.name,
)
)]
pub async fn subscribe(
    form: web::Form<SubscriptionForm>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    domain: web::Data<ApplicationBaseUrl>,
) -> impl Responder {
    let new_subscriber = match form.0.try_into() {
        Ok(name) => name,
        Err(_) => return HttpResponse::BadRequest(),
    };

    let subscriber_id = match insert_subscriber(&pool, &new_subscriber).await {
        Ok(subscriber_id) => subscriber_id,
        Err(_) => return HttpResponse::InternalServerError(),
    };

    let subscription_token = generate_subscription_token();
    if insert_token(&pool, subscriber_id, &subscription_token)
        .await
        .is_err()
    {
        return HttpResponse::InternalServerError();
    }

    if send_confirmation_email(
        &email_client,
        new_subscriber,
        &domain.0,
        &subscription_token,
    )
    .await
    .is_err()
    {
        tracing::error!("Failed to send email request");
        return HttpResponse::InternalServerError();
    }

    HttpResponse::Ok()
}

#[tracing::instrument(
    name = "Store subscriber's token",
    skip(pool, subscriber_id, subscriber_token)
)]
pub async fn insert_token(
    pool: &PgPool,
    subscriber_id: Uuid,
    subscriber_token: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"INSERT INTO subscriptions_tokens(subscription_token, subscription_id) VALUES ($1, $2)"#,
        subscriber_token,
        subscriber_id
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to executre query: {:?}", e);
        e
    })?;

    Ok(())
}

#[tracing::instrument(
    name = "Send confirmation email to a new subscriber",
    skip(email_client, new_subscriber, domain, token)
)]
pub async fn send_confirmation_email(
    email_client: &EmailClient,
    new_subscriber: NewSubscriber,
    domain: &str,
    token: &str,
) -> Result<(), reqwest::Error> {
    let confirmation_link = format!(
        "{}/subscriptions/confirm?subscription_token={}",
        domain, token,
    );

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
) -> Result<Uuid, sqlx::Error> {
    // insert record into database.
    let subscription_id = Uuid::new_v4();
    sqlx::query!(
        r#"
    INSERT INTO subscriptions (id, email, name, subscribed_at, status)
    VALUES ($1,$2,$3,$4,'pending_confirmation')
    "#,
        subscription_id,
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

    Ok(subscription_id)
}

fn generate_subscription_token() -> String {
    let mut rng = thread_rng();
    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
}
