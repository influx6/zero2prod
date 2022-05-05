use std::net::TcpListener;

use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
use sqlx::PgPool;
use tracing_actix_web::TracingLogger;

use crate::routes::health::health_check;
use crate::routes::subscriptions::subscribe;

pub fn run(listener: TcpListener, db_connection: PgPool) -> Result<Server, std::io::Error> {
    let connection = web::Data::new(db_connection);
    Ok(HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .route("/health", web::get().to(health_check))
            .route("/subscriptions", web::post().to(subscribe))
            .app_data(connection.clone())
    })
    .listen(listener)?
    .run())
}
