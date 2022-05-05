use std::net::TcpListener;

use sqlx::PgPool;
use tracing;
use tracing::subscriber::set_global_default;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};

use zero2prod::config::get_configuration;
use zero2prod::run::run;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // redirect all logs events to our subscriber.
    LogTracer::init().expect("Failed to set logger");

    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let formatting_layer = BunyanFormattingLayer::new("zero2prod".into(), std::io::stdout);
    let subscriber = Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(formatting_layer);

    set_global_default(subscriber).expect("Failed to set subscriber");

    let configuration = get_configuration().expect("Should have loaded configuration");
    let db_connection = PgPool::connect(configuration.database.connection_string().as_str())
        .await
        .expect("failed to connect to postgres.");
    let listener = TcpListener::bind(format!("{}:0", configuration.host))
        .expect("failed to bind to random port");

    tracing::info!(
        "Starting service on address: {}",
        listener.local_addr().unwrap()
    );

    run(listener, db_connection)?.await
}
