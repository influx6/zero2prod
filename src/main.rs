use zero2prod::config::get_configuration;
use zero2prod::startup::AppServer;
use zero2prod::telemetry::{get_subscriber, init_subscriber};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    init_subscriber(get_subscriber(
        "zero2prod".into(),
        "info".into(),
        std::io::stdout,
    ));

    let configuration = get_configuration().expect("Should have loaded configuration");
    let server = AppServer::build(configuration)
        .await
        .expect("should have created server");

    server.run_until_stopped().await?;

    Ok(())
}
