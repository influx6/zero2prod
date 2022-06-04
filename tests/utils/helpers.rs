use argon2::password_hash::SaltString;
use argon2::{Algorithm, Argon2, Params, PasswordHasher, Version};
use once_cell::sync::Lazy;
use reqwest::Url;
use sha3::Digest;
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

pub struct ConfirmationLinks {
    pub html: reqwest::Url,
    pub plain_text: reqwest::Url,
}

pub struct TestUser {
    pub user_id: Uuid,
    pub username: String,
    pub password: String,
}

impl TestUser {
    pub fn generate() -> Self {
        Self {
            user_id: Uuid::new_v4(),
            username: Uuid::new_v4().to_string(),
            password: Uuid::new_v4().to_string(),
        }
    }

    async fn store(&self, pool: &PgPool) {
        let salt = SaltString::generate(&mut rand::thread_rng());
        let password_hash = Argon2::new(
            Algorithm::Argon2id,
            Version::V0x13,
            Params::new(15000, 2, 1, None).unwrap(),
        )
        .hash_password(self.password.as_bytes(), &salt)
        .unwrap()
        .to_string();

        sqlx::query!(
            "INSERT INTO users (user_id, username, password_hash) VALUES ($1, $2, $3)",
            Uuid::new_v4(),
            Uuid::new_v4().to_string(),
            password_hash,
        )
        .execute(pool)
        .await
        .expect("Failed to create test users.");
    }
}

pub struct TestApp {
    pub config: Configuration,
    pub email_server: MockServer,
    pub addr: String,
    pub port: u16,
    pub pool: PgPool,
    pub test_user: TestUser,
    pub api_client: reqwest::Client,
}

impl TestApp {
    pub async fn post_login<Body>(&self, body: &Body) -> reqwest::Response
    where
        Body: serde::Serialize,
    {
        self.api_client
            .post(&format!("{}/login", &self.addr))
            .form(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn get_login_html(&self) -> String {
        self.api_client
            .get(&format!("{}/login", &self.addr))
            .send()
            .await
            .expect("Failed to execute request")
            .text()
            .await
            .unwrap()
    }

    pub async fn post_newsletters(&self, body: serde_json::Value) -> reqwest::Response {
        self.api_client
            .post(&format!("{}/newsletters", &self.addr))
            .json(&body)
            .basic_auth(&self.test_user.username, Some(&self.test_user.password))
            .send()
            .await
            .expect("Failed to execute request")
    }

    pub async fn post_subscriptions(&self, body: String) -> reqwest::Response {
        self.api_client
            .post(&format!("{}/subscriptions", &self.addr))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request")
    }

    pub fn get_confirmation_links(&self, email_request: &wiremock::Request) -> ConfirmationLinks {
        let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();

        // extract the link from one of the request fields;
        let get_link = |s: &str| {
            let links: Vec<_> = linkify::LinkFinder::new()
                .links(s)
                .filter(|l| *l.kind() == linkify::LinkKind::Url)
                .collect();
            assert_eq!(links.len(), 1);

            let raw_link = links[0].as_str().to_owned();
            let mut confirmation_link = Url::parse(&raw_link).unwrap();
            assert_eq!(confirmation_link.host_str().unwrap(), "127.0.0.1");
            confirmation_link.set_port(Some(self.port)).unwrap();
            confirmation_link
        };

        let html = get_link(&body["HtmlBody"].as_str().unwrap());
        let plain_text = get_link(&body["TextBody"].as_str().unwrap());

        ConfirmationLinks { html, plain_text }
    }
}

pub fn assert_is_redirect_to(response: &reqwest::Response, location: &str) {
    assert_eq!(response.status().as_u16(), 303);
    assert_eq!(response.headers().get("Location").unwrap(), location);
}

pub async fn spawn_app() -> TestApp {
    Lazy::force(&TRACING);

    let email_server = MockServer::start().await;
    let email_server_url = email_server.uri();

    let api_client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .cookie_store(true)
        .build()
        .unwrap();

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

    let application_port = server.port();
    let addr = format!("http://{}", server.to_server_address());
    let _ = tokio::spawn(server.run_until_stopped());

    let pool = get_connection_pool(&configuration.database);

    let test_user = TestUser::generate();
    test_user.store(&pool).await;

    TestApp {
        pool,
        addr,
        test_user,
        api_client,
        email_server,
        port: application_port,
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
