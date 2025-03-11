mod api_environment;
pub use api_environment::*;
mod uptime;
use lambda_http::tracing::Level;
use tower_http::trace::TraceLayer;
use tower_http::trace::{
	self,
};
pub use uptime::*;

use axum::Router;
use axum::extract::State;
use axum::response::Html;
use axum::routing::get;
use http::StatusCode;

pub fn state_utils_routes() -> Router {
	Router::new()
		.route("/app-info", get(app_info))
		.route("/health-check", get(health_check))
		.with_state(Uptime::new())
		.layer(
			TraceLayer::new_for_http()
				.make_span_with(
					trace::DefaultMakeSpan::new().level(Level::INFO),
				)
				.on_response(
					trace::DefaultOnResponse::new().level(Level::INFO),
				),
		)
}

async fn app_info(State(uptime): State<Uptime>) -> Html<String> {
	// let version = CargoManifest::bevyhub_repo_crate_version();
	let name = std::env::var("CARGO_PKG_NAME").unwrap_or("unknown".into());
	let version =
		std::env::var("CARGO_PKG_VERSION").unwrap_or("unknown".into());
	Html(format!(
		r#"
<h1>App Info</h1>
<p>Name: {}</p>
<p>Version: {}</p>
<p>{}</p>
"#,
		name,
		version,
		uptime.stats()
	))
}


async fn health_check() -> (StatusCode, String) {
	let health = true;
	match health {
		true => (StatusCode::OK, "Healthy".to_string()),
		false => (StatusCode::INTERNAL_SERVER_ERROR, "Not healthy".to_string()),
	}
}


#[cfg(test)]
mod test {
	use sweet::prelude::*;

	#[test]
	fn env_vars() {
		expect(env!("CARGO_PKG_NAME")).to_be("beet_server");
		expect(env!("CARGO_PKG_VERSION")).to_start_with("0.0.");
	}
}
