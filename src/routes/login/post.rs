use std::fmt::Formatter;

use actix_session::Session;
use actix_web::body::BoxBody;
use actix_web::cookie::Cookie;
use actix_web::error::InternalError;
use actix_web::http::header::ContentType;
use actix_web::http::StatusCode;
use actix_web::{web, HttpResponse, ResponseError};
use actix_web_flash_messages::FlashMessage;
use hmac::{Hmac, Mac};
use reqwest::header::LOCATION;
use secrecy::ExposeSecret;
use secrecy::Secret;
use sqlx::PgPool;
use tracing;

use crate::authentication::auth::{validate_credentials, AuthError, Credentials};
use crate::domain::application::HmacSecret;
use crate::session_state::TypedSession;
use crate::utils::error_helpers::error_chain_fmt;

#[derive(thiserror::Error)]
pub enum LoginError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),

    #[error("Something went wrong")]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for LoginError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for LoginError {
    fn status_code(&self) -> StatusCode {
        StatusCode::SEE_OTHER
    }
}

#[derive(serde::Deserialize)]
pub struct FormData {
    username: String,
    password: Secret<String>,
}

fn login_redirect(e: LoginError) -> InternalError<LoginError> {
    FlashMessage::error(e.to_string()).send();
    let response = HttpResponse::SeeOther()
        .insert_header((LOCATION, "/login"))
        .finish();
    InternalError::from_response(e, response)
}

#[tracing::instrument(
skip(form, pool, session),
fields(username = tracing::field::Empty, user_id = tracing::field::Empty)
)]
pub async fn login(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>,
    session: TypedSession,
    // secret: web::Data<HmacSecret>,
) -> Result<HttpResponse, InternalError<LoginError>> {
    let credentials = Credentials {
        username: form.0.username,
        password: form.0.password,
    };

    tracing::Span::current().record("username", &tracing::field::display(&credentials.username));
    match validate_credentials(credentials, &pool).await {
        Ok(user_id) => {
            tracing::Span::current().record("user_id", &tracing::field::display(&user_id));

            session.renew();
            session
                .insert_user_id(user_id)
                .map_err(|e| login_redirect(LoginError::UnexpectedError(e.into())))?;

            Ok(HttpResponse::SeeOther()
                .insert_header((LOCATION, "/admin/dashboard"))
                .finish())
        }
        Err(e) => {
            let e = match e {
                AuthError::InvalidCredentials(_) => LoginError::AuthError(e.into()),
                AuthError::UnexpectedError(_) => LoginError::UnexpectedError(e.into()),
            };

            // let query_string = format!("error={}", urlencoding::Encoded::new(e.to_string()));
            //
            // let hmac_tag = {
            //     let mut mac =
            //         Hmac::<sha2::Sha256>::new_from_slice(secret.0.expose_secret().as_bytes())
            //             .unwrap();
            //     mac.update(query_string.as_bytes());
            //     mac.finalize().into_bytes()
            // };

            // FlashMessage::error(e.to_string()).send();
            //
            // // let target_location = format!("/login?{}&tag={:x}", query_string, hmac_tag);
            // let response = HttpResponse::SeeOther()
            //     .insert_header((LOCATION, "/login"))
            //     .cookie(Cookie::new("_flash", e.to_string()))
            //     .finish();
            //
            // // wraps our expected error still providing context on error but also
            // // a response to be sent to the caller of request.
            // Err(InternalError::from_response(e, response))
            Err(login_redirect(e))
        }
    }
}
