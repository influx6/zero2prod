use crate::utils::helpers::spawn_app;

#[tokio::test]
async fn requests_missing_authentication_are_rejected() {
    // arrange
    let app = spawn_app().await;

    let response = reqwest::Client::new()
        .post(&format!("{}/newsletters", &app.addr))
        .json(&serde_json::json!({
            "title": "Newsletter title",
            "content": {
                    "text": "Newsletter body in plain text",
                    "html": "<p>Newsletter body in html</p>",
            }
        }))
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(401, response.status().as_u16());
    assert_eq!(
        r#"Basic realm="publish""#,
        response.headers()["WWW-Authenticate"]
    );
}
