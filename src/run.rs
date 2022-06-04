use std::net::TcpListener;

use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
use secrecy::Secret;
use sqlx::PgPool;
use tracing_actix_web::TracingLogger;

use crate::domain::application::{ApplicationBaseUrl, HmacSecret};
use crate::mail::send_email::EmailClient;
use crate::routes::health::health_check;
use crate::routes::home::home;
use crate::routes::login::{get::login_form, post::login};
use crate::routes::newsletter::publish_newsletter;
use crate::routes::subscription_confirm::confirm;
use crate::routes::subscriptions::subscribe;

pub fn run(
    listener: TcpListener,
    db_connection: PgPool,
    email_client: EmailClient,
    domain: String,
    hmac_secret: HmacSecret,
) -> Result<Server, std::io::Error> {
    let hmac_data = web::Data::new(HmacSecret(hmac_secret.0.clone()));
    let connection = web::Data::new(db_connection);
    let email_client_data = web::Data::new(email_client);
    let domain_url = web::Data::new(ApplicationBaseUrl(domain));
    Ok(HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            // get endpoints
            .route("/", web::get().to(home))
            .route("/login", web::get().to(login_form))
            .route("/health", web::get().to(health_check))
            // post endpoints
            .route("/login", web::post().to(login))
            .route("/subscriptions", web::post().to(subscribe))
            .route("/newsletters", web::post().to(publish_newsletter))
            .route("/subscriptions/confirm", web::get().to(confirm))
            .app_data(connection.clone())
            .app_data(email_client_data.clone())
            .app_data(domain_url.clone())
            .app_data(hmac_data.clone())
    })
    .listen(listener)?
    .run())
}
