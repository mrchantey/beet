use tower_http::classify::ServerErrorsAsFailures;
use tower_http::classify::SharedClassifier;
use tower_http::trace::DefaultMakeSpan;
use tower_http::trace::TraceLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

pub fn tracing_layer() -> TraceLayer<SharedClassifier<ServerErrorsAsFailures>> {
	tracing_subscriber::registry()
		.with(
			tracing_subscriber::EnvFilter::try_from_default_env()
				.unwrap_or_else(|_| {
					"example_websockets=debug,tower_http=debug".into()
				}),
		)
		.with(tracing_subscriber::fmt::layer())
		.init();

	TraceLayer::new_for_http()
		.make_span_with(DefaultMakeSpan::default().include_headers(true))
}
