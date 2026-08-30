use crate::prelude::*;
use beet_core::prelude::*;
use bytes::Bytes;
use lambda_http::request::RequestContext;
use lambda_http::tower::service_fn;

impl HttpServer {
	/// Sets up the Lambda runtime and runs the provided handler indefinitely.
	///
	/// The lambda runtime owns the lifecycle (it stops invoking when the function is
	/// torn down) and there is no listener to close, so the shutdown signal is unused.
	pub async fn start_lambda(
		entity: AsyncEntity,
		_shutdown: OnceValueRx<()>,
	) -> Result {
		// This variable only applies to API Gateway stages,
		// you can remove it if you don't use them.
		// i.e
		// - default: `GET /test-stage/todo/id/123`
		// - ignored: `GET /todo/id/123`
		unsafe {
			env_ext::set_var("AWS_LAMBDA_HTTP_IGNORE_STAGE_IN_PATH", "true")
		}?;
		// required to enable CloudWatch error logging by the runtime
		// tracing::init_default_subscriber(); //we use PrettyTracing instead

		info!("🌱 listening for lambda requests");

		// lambda_http uses Tokio internally, so we need a Tokio runtime context.
		// The enter guard sets the Tokio reactor for I/O operations while
		// beet's async-executor drives the future. The guard is `!Send` and held
		// across the await, which is fine: `HttpServerFn` runs on the local thread
		// (its future is a `LocalBoxedFuture`), never moved between threads.
		let _guard = async_ext::tokio().enter();

		lambda_http::run(service_fn(move |lambda_req| {
			let entity = entity.clone();
			handle_request(entity, lambda_req)
		}))
		.await
		.map_err(|err| {
			error!("Error running lambda: {:?}", err);
			bevyhow!("{}", err)
		})
	}
}

/// Handler function that processes each invocation.
///
/// Two event shapes arrive here and they fail differently. An http event (api
/// gateway, function url, load balancer) has a client waiting, so a failed
/// dispatch is answered with a 500 and the invocation still succeeded. A
/// [`ScheduledInvoke`] has no client at all: its dispatch IS the invocation, so
/// a failure must fail the invocation, or a job that has been broken for weeks
/// reports green in every metric the schedule publishes.
async fn handle_request(
	entity: AsyncEntity,
	lambda_req: lambda_http::Request,
) -> Result<lambda_http::Response<lambda_http::Body>, lambda_http::Error> {
	if is_scheduled(&lambda_req) {
		return handle_scheduled(entity, lambda_req).await.map_err(|err| {
			error!("Scheduled invoke failed: {err}");
			lambda_http::Error::from(err.to_string())
		});
	}

	let result: Result<lambda_http::Response<lambda_http::Body>> = async {
		let req = lambda_to_request(lambda_req)?;
		let res = entity.exchange_child(req).await;
		response_to_lambda(res).await
	}
	.await;

	match result {
		Ok(lambda_res) => Ok(lambda_res),
		Err(e) => {
			error!("Failed to process lambda request: {}", e);
			Ok(lambda_http::Response::builder()
				.status(500)
				// don't leak internal error context to client
				.body(lambda_http::Body::Text(
					"Internal Server Error".to_string(),
				))
				.unwrap())
		}
	}
}

/// Whether this event is not http-shaped, ie a scheduled invoke.
///
/// A schedule delivers its own json payload verbatim, which no http event shape
/// parses, so `lambda_http` hands it over as a synthetic `POST` marked
/// [`RequestContext::PassThrough`]. That marker is the seam: everything the
/// runtime recognized as http keeps its own context.
fn is_scheduled(lambda_req: &lambda_http::Request) -> bool {
	matches!(
		lambda_req.extensions().get::<RequestContext>(),
		Some(RequestContext::PassThrough)
	)
}

/// Dispatch a [`ScheduledInvoke`]'s request and resolve the invocation with it,
/// through the same status ladder a one-shot command exits by.
///
/// The response body is echoed back as the invocation's result so a run's own
/// report lands in the logs, but nothing consumes it: the outcome is the status.
async fn handle_scheduled(
	entity: AsyncEntity,
	lambda_req: lambda_http::Request,
) -> Result<lambda_http::Response<lambda_http::Body>> {
	let payload = lambda_body_bytes(lambda_req.into_body());
	let invoke = ScheduledInvoke::from_payload(&payload)?;
	let path = invoke.path().clone();
	info!("⏱ scheduled invoke: {} {path}", invoke.method());

	let (parts, body) = entity
		.exchange_child(invoke.into_request())
		.await
		.into_parts();
	let body = body.into_bytes().await?;
	let text = String::from_utf8_lossy(&body);
	if parts.status_to_exit_code().is_err() {
		bevybail!(
			"the scheduled invoke of `{path}` failed with {}: {text}",
			parts.status()
		);
	}
	lambda_http::Response::builder()
		.status(parts.status().as_u16())
		// a pass-through response is parsed as json, so a non-json body would
		// serialize as `null` rather than the run's report
		.body(lambda_http::Body::Text(
			serde_json::json!({ "path": path, "result": text }).to_string(),
		))?
		.xok()
}

/// Convert lambda HTTP request to beet Request
fn lambda_to_request(lambda_req: lambda_http::Request) -> Result<Request> {
	let (parts, lambda_body) = lambda_req.into_parts();
	Request::from_parts(
		parts.into(),
		Body::Bytes(lambda_body_bytes(lambda_body)),
	)
	.xok()
}

/// The bytes a lambda body carries. Request streaming is not supported in
/// lambda; `Body` is non_exhaustive, so an unknown future variant reads as an
/// empty body rather than a panic.
fn lambda_body_bytes(body: lambda_http::Body) -> Bytes {
	match body {
		lambda_http::Body::Text(text) => Bytes::from(text),
		lambda_http::Body::Binary(binary) => Bytes::from(binary),
		_ => Bytes::new(),
	}
}

/// Convert beet Response to lambda HTTP response
async fn response_to_lambda(
	beet_res: Response,
) -> Result<lambda_http::Response<lambda_http::Body>> {
	// Response streaming not supported in lambda
	let (parts, body) = beet_res.into_parts();
	let bytes = body.into_bytes().await?;

	// Convert bytes to lambda Body
	let lambda_body = if bytes.is_empty() {
		lambda_http::Body::Empty
	} else {
		match String::from_utf8(bytes.to_vec()) {
			Ok(text) => lambda_http::Body::Text(text),
			Err(_) => lambda_http::Body::Binary(bytes.to_vec()),
		}
	};
	let http_parts = parts.try_into()?;
	lambda_http::Response::from_parts(http_parts, lambda_body).xok()
}

#[cfg(test)]
mod test {
	use super::*;

	/// The event a schedule delivers, as the runtime hands it over: not http
	/// shaped, so the payload is the whole body.
	fn scheduled_event(payload: &str) -> lambda_http::Request {
		serde_json::from_str::<lambda_http::request::LambdaRequest>(payload)
			.unwrap()
			.into()
	}

	/// The seam the adapter branches on: a schedule's own payload matches no
	/// http shape and arrives marked pass-through, while a gateway event does
	/// not.
	#[beet_core::test]
	fn tells_a_scheduled_invoke_from_an_http_event() {
		let payload =
			serde_json::to_string(&ScheduledInvoke::new("analytics/rollup"))
				.unwrap();
		let event = scheduled_event(&payload);
		is_scheduled(&event).xpect_true();
		ScheduledInvoke::from_payload(&lambda_body_bytes(event.into_body()))
			.unwrap()
			.into_request()
			.path()
			.xpect_eq(&["analytics", "rollup"]);

		is_scheduled(&scheduled_event(
			r#"{"httpMethod":"GET","path":"/","requestContext":{"elb":{"targetGroupArn":"arn"}}}"#,
		))
		.xpect_false();
	}
}
