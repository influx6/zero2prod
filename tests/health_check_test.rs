use crate::helpers::spawn_app;

mod helpers;

#[tokio::test]
async fn health_check_endpoint_returns_200() {
    let app = spawn_app().await;

    let client = reqwest::Client::new();

    let response = client
        .get(format!("{}/health", app.addr))
        .send()
        .await
        .expect("Failed to execute request.");

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}
