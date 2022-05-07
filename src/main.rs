use std::net::TcpListener;

use sqlx::postgres::PgPoolOptions;
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
    let db_connection = PgPoolOptions::new()
        .connect_timeout(std::time::Duration::from_secs(2))
        .connect_lazy_with(configuration.database.with_db());
    let listener = TcpListener::bind(format!(
        "{}:{}",
        configuration.app.host, configuration.app.port
    ))
    .expect("failed to bind to random port");

    tracing::info!(
        "Starting service on address: {}",
        listener.local_addr().unwrap()
    );

    run(listener, db_connection)?.await
}
