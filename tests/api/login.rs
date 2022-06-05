//! tests/api/helpers.rs

use std::collections::HashSet;

use reqwest::header::HeaderValue;

use crate::utils::helpers::{assert_is_redirect_to, spawn_app};

#[tokio::test]
async fn an_error_flash_message_is_set_on_failure() {
    // arrange
    let app = spawn_app().await;

    // Act
    let login_body = serde_json::json!({
        "username":  "random-username",
        "password":  "random-username",
    });

    let response = app.post_login(&login_body).await;

    // Assert
    assert_eq!(response.status().as_u16(), 303);
    assert_is_redirect_to(&response, "/login");

    // let cookies: HashSet<_> = response
    //     .headers()
    //     .get_all("Set-Cookie")
    //     .into_iter()
    //     .collect();
    // assert!(cookies.contains(&HeaderValue::from_str("_flash=Authentication failed").unwrap()));
    let flash_cookie = response.cookies().find(|c| c.name() == "_flash").unwrap();
    assert_eq!(flash_cookie.value(), "Authentication failed");

    // Act 2: follow redirect
    let html_page = app.get_login_html().await;
    assert!(html_page.contains(r#"<p><i>Authentication failed<i></p>"#));

    // Act 3: ensure error message is removed
    let html_page = app.get_login_html().await;
    assert!(!html_page.contains(r#"<p><i>Authentication failed<i></p>"#));
}
