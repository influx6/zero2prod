use std::net::TcpListener;

use actix_web::web::Data;
use once_cell::sync::Lazy;
use secrecy::ExposeSecret;
use sqlx::postgres::PgQueryResult;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;

use zero2prod::config::{get_configuration, Configuration, DatabaseSettings};
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
    pub addr: String,
    pub pool: PgPool,
}

pub async fn spawn_app() -> TestApp {
    Lazy::force(&TRACING);

    let mut configuration = get_configuration().expect("should load configuration");

    let db_name = Uuid::new_v4().to_string();
    configuration.database.database_name = db_name;
    let db_connection = configure_database(&configuration.database).await;

    let listener = TcpListener::bind(format!("{}:0", configuration.app.host.clone()))
        .expect("failed to bind to random port");
    let port = listener.local_addr().unwrap().port();
    let server =
        zero2prod::run::run(listener, db_connection.clone()).expect("Failed to bind address");
    let _ = tokio::spawn(server);

    let hostname = configuration.app.host.clone();
    TestApp {
        config: configuration,
        pool: db_connection.clone(),
        addr: format!("http://{}:{}", hostname, port),
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
