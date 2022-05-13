use once_cell::sync::Lazy;
use sqlx::postgres::PgQueryResult;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;
use wiremock::MockServer;

use zero2prod::config::{get_configuration, Configuration, DatabaseSettings};
use zero2prod::startup::{get_connection_pool, AppServer};
use zero2prod::telemetry::{get_subscriber, init_subscriber};

static TRACING: Lazy<()> = Lazy::new(|| {
    let default_filter_level = "info".to_string();
    let subscriber_name = "test".to_string();
    if std::env::var("TEST_LOG").is_ok() {
        init_subscriber(get_subscriber(
            "test".into(),
            "debug".into(),
            std::io::stdout,
        ));
    } else {
        init_subscriber(get_subscriber("test".into(), "debug".into(), std::io::sink));
    }
});

pub struct TestApp {
    pub config: Configuration,
    pub email_server: MockServer,
    pub addr: String,
    pub pool: PgPool,
}

impl TestApp {
    pub async fn post_subscriptions(&self, body: String) -> reqwest::Response {
        reqwest::Client::new()
            .post(&format!("{}/subscriptions", self.addr))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request")
    }
}

pub async fn spawn_app() -> TestApp {
    Lazy::force(&TRACING);

    let email_server = MockServer::start().await;
    let email_server_url = email_server.uri();

    let configuration = {
        let mut c = get_configuration().expect("should load configuration");
        let db_name = Uuid::new_v4().to_string();

        c.email_client.base_url = email_server_url.into();
        c.database.database_name = db_name;
        c.app.port = 0;
        c
    };

    // configure and migrate database
    configure_database(&configuration.database).await;

    let server = AppServer::build(configuration.clone())
        .await
        .expect("should have created server");

    let addr = format!("http://{}", server.to_server_address());
    let _ = tokio::spawn(server.run_until_stopped());

    let pool = get_connection_pool(&configuration.database);

    TestApp {
        pool,
        addr,
        email_server,
        config: configuration,
    }
}

pub async fn configure_database(database_settings: &DatabaseSettings) -> PgPool {
    let mut db_connection = PgConnection::connect_with(&database_settings.without_db())
        .await
        .expect("failed to connect to postgres.");

    db_connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, database_settings.database_name).as_str())
        .await
        .expect("Failed to create database");

    // Migrate database
    let db_pool = PgPool::connect_with(database_settings.with_db())
        .await
        .expect("failed to connect to postgres.");

    sqlx::migrate!("./migrations")
        .run(&db_pool)
        .await
        .expect("Failed to migrate the database");

    return db_pool;
}

pub async fn drop_table(pool: &PgPool) -> sqlx::Result<PgQueryResult> {
    sqlx::query!(
        r#"
        DO $$ DECLARE
            r RECORD;
        BEGIN
            -- if the schema you operate on is not "current", you will want to
            -- replace current_schema() in query with 'schematodeletetablesfrom'
            -- *and* update the generate 'DROP...' accordingly.
            FOR r IN (SELECT tablename FROM pg_tables WHERE schemaname = current_schema()) LOOP
                EXECUTE 'DROP TABLE IF EXISTS ' || quote_ident(r.tablename) || ' CASCADE';
            END LOOP;
        END $$;
    "#
    )
    .execute(pool)
    .await
}
