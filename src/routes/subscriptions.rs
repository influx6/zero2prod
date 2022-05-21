use std::any::TypeId;
use std::error::Error;
use std::fmt::{Debug, Formatter};

use actix_web::body::BoxBody;
use actix_web::error::ParseError::Status;
use actix_web::http::StatusCode;
use actix_web::{web, HttpResponse, Responder, ResponseError};
use chrono::Utc;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use sqlx::{PgPool, Postgres, Transaction};
use tracing;
use uuid::Uuid;

use crate::domain::application::ApplicationBaseUrl;
use crate::domain::new_subscriber::NewSubscriber;
use crate::domain::subscriber_email::SubscriberEmail;
use crate::domain::subscriber_name::SubscriberName;
use crate::mail::send_email::EmailClient;

fn error_chain_fmt(
    e: &impl std::error::Error,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    writeln!(f, "{}\n", e)?;
    let mut current = e.source();
    while let Some(cause) = current {
        writeln!(f, "Caused by:\n\t{}", cause)?;
        current = cause.source();
    }
    Ok(())
}

pub struct StoreTokenError(sqlx::Error);

impl std::fmt::Display for StoreTokenError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "A database error was encountered while \
            trying to store a subscription token.",
        )
    }
}

impl std::fmt::Debug for StoreTokenError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl std::error::Error for StoreTokenError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.0)
    }
}

#[derive(thiserror::Error)]
pub enum SubscriberError {
    #[error("{0}")]
    ValidationError(String),

    // Transparent delegates both `Display` and `source` implementation to the type wrapped by `Unexpected`.
    #[error("{1}")]
    UnexpectedError(#[source] Box<dyn std::error::Error>, String),
}

impl std::fmt::Debug for SubscriberError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for SubscriberError {
    fn status_code(&self) -> StatusCode {
        match self {
            SubscriberError::ValidationError(_) => StatusCode::BAD_REQUEST,
            SubscriberError::UnexpectedError(_, _) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

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
) -> Result<impl Responder, SubscriberError> {
    let new_subscriber = form
        .0
        .try_into()
        .map_err(SubscriberError::ValidationError)?;

    let mut transaction = pool.begin().await.map_err(|e| {
        SubscriberError::UnexpectedError(
            Box::new(e),
            "Failed to acquire a postgres connection from the pool".into(),
        )
    })?;

    let subscriber_id = insert_subscriber(&mut transaction, &new_subscriber)
        .await
        .map_err(|e| {
            SubscriberError::UnexpectedError(
                Box::new(e),
                "Failed to insert new subscriber into the database".into(),
            )
        })?;

    let subscription_token = generate_subscription_token();

    insert_token(&mut transaction, subscriber_id, &subscription_token)
        .await
        .map_err(|e| {
            SubscriberError::UnexpectedError(
                Box::new(e),
                "Failed to store confirmation token for a new subscriber".into(),
            )
        })?;

    transaction.commit().await.map_err(|e| {
        SubscriberError::UnexpectedError(
            Box::new(e),
            "Failed to commit SQL transaction to store a new subscriber".into(),
        )
    })?;

    send_confirmation_email(
        &email_client,
        new_subscriber,
        &domain.0,
        &subscription_token,
    )
    .await
    .map_err(|e| {
        SubscriberError::UnexpectedError(Box::new(e), "Failed to send confirmation email".into())
    })?;

    Ok(HttpResponse::Ok())
}

#[tracing::instrument(
    name = "Store subscriber's token",
    skip(transaction, subscriber_id, subscriber_token)
)]
pub async fn insert_token(
    transaction: &mut Transaction<'_, Postgres>,
    subscriber_id: Uuid,
    subscriber_token: &str,
) -> Result<(), StoreTokenError> {
    sqlx::query!(
        r#"INSERT INTO subscriptions_tokens(subscription_token, subscription_id) VALUES ($1, $2)"#,
        subscriber_token,
        subscriber_id
    )
    .execute(transaction)
    .await
    .map_err(|e| {
        tracing::error!("Failed to executre query: {:?}", e);
        StoreTokenError(e)
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
    skip(subscriber, transaction)
)]
pub async fn insert_subscriber(
    transaction: &mut Transaction<'_, Postgres>,
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
    .execute(transaction)
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
