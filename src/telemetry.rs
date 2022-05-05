use tracing::subscriber::set_global_default;
use tracing::Subscriber;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};

/// Composes multiple layers into a tracing subscribers to collect telemetry.
///
/// # Implementation Notes
///
/// We are using `impl Subscriber` as a return type to avoid having to spell out
/// the actual type of the returned subscriber, which is quite complex.
/// We need to explicitly call out that the returned subscriber is `Send` and `Sync`
/// to make it possible to pass it to `init_subscriber` later on.
pub fn get_subscriber<Sink>(
    name: String,
    filter: String,
    sink: Sink,
) -> impl Subscriber + Send + Sync
where
    // This "weird" syntax is a higher ranked trait bound.
    //  It basically means that Sink implements the MakeWriter trait fro all choices
    // of the lifetime parameter 'a.
    Sink: for<'a> MakeWriter<'a> + Send + Sync + 'static,
{
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(filter));
    let formatting_layer = BunyanFormattingLayer::new(name, sink);

    // return a subscriber that combines multiple layers for logging.
    Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(formatting_layer)
}

pub fn init_subscriber(subscriber: impl Subscriber + Send + Sync) {
    // redirect all logs events to our subscriber.
    LogTracer::init().expect("Failed to set logger");
    set_global_default(subscriber).expect("Failed to set subscriber");
}
