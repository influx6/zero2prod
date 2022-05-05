use std::net::TcpListener;

use secrecy::ExposeSecret;
use sqlx::PgPool;
use tracing;

use zero2prod::config::get_configuration;
use zero2prod::run::run;
use zero2prod::telemetry::{get_subscriber, init_subscriber};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    init_subscriber(get_subscriber(
        "zero2prod".into(),
        "info".into(),
        std::io::stdout,
    ));

    let configuration = get_configuration().expect("Should have loaded configuration");
    let db_connection = PgPool::connect(
        configuration
            .database
            .connection_string()
            .expose_secret()
            .as_str(),
    )
    .await
    .expect("failed to connect to postgres.");
    let listener = TcpListener::bind(format!("{}:{}", configuration.host, configuration.port))
        .expect("failed to bind to random port");

    tracing::info!(
        "Starting service on address: {}",
        listener.local_addr().unwrap()
    );

    run(listener, db_connection)?.await
}
