use beet_core::prelude::*;
use lambda_http::IntoResponse;
use lambda_http::Request;
use lambda_http::Service;
use lambda_http::lambda_runtime::Diagnostic;
use lambda_http::tracing;

/// Sets up the Lambda runtime and runs the provided handler indefinitely.
pub async fn run_lambda<'a, R, S, E>(handler: S) -> Result
where
	S: Service<Request, Response = R, Error = E>,
	S::Future: Send + 'a,
	R: IntoResponse,
	E: std::fmt::Debug + Into<Diagnostic>,
{
	// This variable only applies to API Gateway stages,
	// you can remove it if you don't use them.
	// i.e with: `GET /test-stage/todo/id/123` without: `GET /todo/id/123`
	unsafe {
		std::env::set_var("AWS_LAMBDA_HTTP_IGNORE_STAGE_IN_PATH", "true");
	};
	// required to enable CloudWatch error logging by the runtime
	// tracing::init_default_subscriber(); //deprecated use PrettyTracing instead

	tracing::info!("🌱 listening for requests");

	lambda_http::run(handler).await.map_err(|err| {
		tracing::error!("Error running lambda: {:?}", err);
		bevyhow!("{}", err)
	})
}
