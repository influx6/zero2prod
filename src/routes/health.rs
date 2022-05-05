use actix_web::{HttpResponse, Responder};
use tracing;
use uuid::Uuid;

pub async fn health_check() -> impl Responder {
    let request_id = Uuid::new_v4();
    let request_span = tracing::info_span!(
        "Checking service health",
        %request_id,
    );

    let _request_span_guard = request_span.enter();
    tracing::info!("Service is healthy!");
    HttpResponse::Ok()
}
