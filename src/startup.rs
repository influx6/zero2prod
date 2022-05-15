use std::net::TcpListener;

use actix_web::dev::Server;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

use crate::config::{Configuration, DatabaseSettings};
use crate::mail::send_email::EmailClient;
use crate::run::run;

pub struct AppServer {
    port: u16,
    address: String,
    server: Server,
}

impl AppServer {
    pub async fn build(configuration: Configuration) -> Result<Self, std::io::Error> {
        let db_connection = get_connection_pool(&configuration.database);

        let listener = TcpListener::bind(format!(
            "{}:{}",
            configuration.app.host, configuration.app.port
        ))
        .expect("failed to bind to random port");

        tracing::info!(
            "Starting service on address: {}",
            listener.local_addr().unwrap()
        );

        let sender_email = configuration
            .email_client
            .sender()
            .expect("invalid sender email address.");
        let email_client = EmailClient::new(configuration.email_client.clone(), sender_email);

        let address = configuration.app.host.clone();
        let port = listener.local_addr().unwrap().port();
        let server = run(
            listener,
            db_connection,
            email_client,
            configuration.app.domain,
        )?;

        Ok(Self {
            port,
            address,
            server,
        })
    }

    pub fn to_server_address(&self) -> String {
        format!("{}:{}", self.address.clone(), self.port.clone())
    }

    pub fn address(&self) -> String {
        self.address.clone()
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server.await
    }
}

pub fn get_connection_pool(database: &DatabaseSettings) -> PgPool {
    PgPoolOptions::new()
        .connect_timeout(std::time::Duration::from_secs(2))
        .connect_lazy_with(database.with_db())
}
