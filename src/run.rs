use std::net::TcpListener;

use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
use sqlx::PgPool;
use tracing_actix_web::TracingLogger;

use crate::mail::send_email::EmailClient;
use crate::routes::health::health_check;
use crate::routes::subscriptions::subscribe;

pub fn run(
    listener: TcpListener,
    db_connection: PgPool,
    email_client: EmailClient,
) -> Result<Server, std::io::Error> {
    let connection = web::Data::new(db_connection);
    let email_client_data = web::Data::new(email_client);
    Ok(HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .route("/health", web::get().to(health_check))
            .route("/subscriptions", web::post().to(subscribe))
            .app_data(connection.clone())
            .app_data(email_client_data.clone())
    })
    .listen(listener)?
    .run())
}
