use std::convert::Infallible;

use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use bytes::Bytes;
use lambda_http::tracing;
use tower::service_fn;



pub fn connect_lambda(mut commands: AsyncCommands) {
	commands.run_local(run_lambda);
}


/// Sets up the Lambda runtime and runs the provided handler indefinitely.
async fn run_lambda(world: AsyncWorld) -> Result {
	// This variable only applies to API Gateway stages,
	// you can remove it if you don't use them.
	// i.e with: `GET /test-stage/todo/id/123` without: `GET /todo/id/123`
	unsafe {
		std::env::set_var("AWS_LAMBDA_HTTP_IGNORE_STAGE_IN_PATH", "true");
	};
	// required to enable CloudWatch error logging by the runtime
	// tracing::init_default_subscriber(); //we use PrettyTracing instead

	tracing::info!("🌱 listening for requests");
	lambda_http::run(service_fn(
		async move |lambda_req| -> Result<
			lambda_http::Response<lambda_http::Body>,
			Infallible,
		> {
			let result: Result<lambda_http::Response<lambda_http::Body>> =
				async move {
					let request = lambda_to_request(lambda_req)?;
					let response = world.oneshot(request).await?;
					response_to_lambda(response).await
				}
				.await;
			// Convert beet response to lambda response
			match result {
				Ok(lambda_res) => Ok(lambda_res),
				Err(e) => {
					tracing::error!("Beet/Lambda conversion failed: {}", e);
					Ok(lambda_http::Response::builder()
						.status(500)
						.body(lambda_http::Body::Text(format!(
							"Internal error",
						)))
						.unwrap())
				}
			}
		},
	))
	.await
	.map_err(|err| {
		tracing::error!("Error running lambda: {:?}", err);
		bevyhow!("{}", err)
	})
}



/// Convert lambda HTTP request to beet Request
fn lambda_to_request(lambda_req: lambda_http::Request) -> Result<Request> {
	let (parts, lambda_body) = lambda_req.into_parts();

	// Convert lambda body to beet Body
	let body = match lambda_body {
		lambda_http::Body::Empty => Body::default(),
		lambda_http::Body::Text(text) => Body::Bytes(Bytes::from(text)),
		lambda_http::Body::Binary(binary) => Body::Bytes(Bytes::from(binary)),
		// Request streaming not supported in lambda
	};

	Ok(Request::from_parts(parts, body))
}

/// Convert beet Response to lambda HTTP response
async fn response_to_lambda(
	beet_res: Response,
) -> Result<lambda_http::Response<lambda_http::Body>> {
	// Response streaming not supported in lambda
	let bytes = beet_res.body.into_bytes().await?;

	// Convert bytes to lambda Body
	let lambda_body = if bytes.is_empty() {
		lambda_http::Body::Empty
	} else {
		match String::from_utf8(bytes.to_vec()) {
			Ok(text) => lambda_http::Body::Text(text),
			Err(_) => lambda_http::Body::Binary(bytes.to_vec()),
		}
	};
	lambda_http::Response::from_parts(beet_res.parts, lambda_body).xok()
}
