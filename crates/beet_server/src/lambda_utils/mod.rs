use lambda_http::Error;
use lambda_http::IntoResponse;
use lambda_http::Request;
use lambda_http::Service;
use lambda_http::lambda_runtime::Diagnostic;
use lambda_http::run;
use lambda_http::tracing;

/// Sets up the Lambda runtime and runs the provided handler indefinitely.
pub async fn run_lambda<'a, R, S, E>(handler: S) -> Result<(), Error>
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
	tracing::init_default_subscriber();

	run(handler).await
}


// pub async fn run<'a, R, S, E>(handler: S) -> Result<(), Error>
// where
//     S: Service<Request, Response = R, Error = E>,
//     S::Future: Send + 'a,
//     R: IntoResponse,
//     E: std::fmt::Debug + Into<Diagnostic>,
