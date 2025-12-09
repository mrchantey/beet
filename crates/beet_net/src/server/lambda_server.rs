use crate::prelude::*;
use crate::server::HandlerFn;
use beet_core::prelude::*;
use bytes::Bytes;
use lambda_http::tower::service_fn;
use lambda_http::tracing;


/// Starts the Lambda runtime for the HttpServer
pub(super) fn start_lambda_server(
	In(entity): In<Entity>,
	query: Query<&HttpServer>,
	mut async_commands: AsyncCommands,
) -> Result {
	let server = query.get(entity)?;
	let handler = server.handler();

	async_commands.run_local(async move |world| -> Result {
		run_lambda(world.entity(entity), handler).await
	});

	Ok(())
}

/// Sets up the Lambda runtime and runs the provided handler indefinitely.
async fn run_lambda(entity: AsyncEntity, handler: HandlerFn) -> Result {
	// This variable only applies to API Gateway stages,
	// you can remove it if you don't use them.
	// i.e with: `GET /test-stage/todo/id/123` without: `GET /todo/id/123`
	unsafe {
		std::env::set_var("AWS_LAMBDA_HTTP_IGNORE_STAGE_IN_PATH", "true");
	};
	// required to enable CloudWatch error logging by the runtime
	// tracing::init_default_subscriber(); //we use PrettyTracing instead

	tracing::info!("🌱 listening for lambda requests");

	lambda_http::run(service_fn(move |lambda_req| {
		let entity = entity.clone();
		let handler = handler.clone();
		handle_request(entity, handler, lambda_req)
	}))
	.await
	.map_err(|err| {
		tracing::error!("Error running lambda: {:?}", err);
		bevyhow!("{}", err)
	})
}


/// Handler function that processes each lambda request
async fn handle_request(
	entity: AsyncEntity,
	handler: HandlerFn,
	lambda_req: lambda_http::Request,
) -> std::result::Result<
	lambda_http::Response<lambda_http::Body>,
	std::convert::Infallible,
> {
	let result: Result<lambda_http::Response<lambda_http::Body>> = async {
		let request = lambda_to_request(lambda_req)?;
		let response = handler(entity, request).await;
		response_to_lambda(response).await
	}
	.await;

	match result {
		Ok(lambda_res) => Ok(lambda_res),
		Err(e) => {
			error!("Failed to process lambda request: {}", e);
			Ok(lambda_http::Response::builder()
				.status(500)
				// dont leak internal error context to client
				.body(lambda_http::Body::Text(
					"Internal Server Error".to_string(),
				))
				.unwrap())
		}
	}
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
