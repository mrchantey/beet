//! The Lambda runtime: beet's own loop over the Runtime API.
use crate::prelude::*;
use base64::Engine;
use beet_action::prelude::*;
use beet_core::prelude::*;
use bytes::Bytes;
use core::str::FromStr;
use serde_json::Value;
use serde_json::json;

impl HttpServer {
	/// Serve Lambda invocations until the function is torn down: the
	/// [`LambdaRuntime`] the task's environment names, dispatching each
	/// invocation to this server's child.
	///
	/// The runtime owns the lifecycle (it stops invoking when the function is
	/// torn down) and there is no listener to close, so the shutdown signal is
	/// unused. A server with no dispatch child cannot answer anything, which
	/// is reported to the runtime as an init failure rather than as a 500 per
	/// invocation.
	pub async fn start_lambda(
		entity: AsyncEntity,
		_shutdown: OnceValueRx<()>,
	) -> Result {
		let runtime = LambdaRuntime::from_env()?;
		let host = entity
			.world()
			.run_system_cached_with::<_, Result<Entity>, _, _>(
				first_matching_child::<Request, Response>,
				entity.id(),
			)
			.await
			.map_err(BevyError::from)
			.flatten();
		if let Err(err) = host {
			runtime.init_error(&err).await?;
			return Err(err);
		}
		info!("🌱 listening for lambda invocations");
		runtime
			.run(move |request| {
				let entity = entity.clone();
				async move { entity.exchange_child(request).await }
			})
			.await
	}
}

/// The Lambda Runtime API at the address the task names
/// (`AWS_LAMBDA_RUNTIME_API`): plain http to a local endpoint, so the loop is
/// [`Request::send`] and nothing else.
///
/// Each invocation is `GET /2018-06-01/runtime/invocation/next`, which
/// long-polls (the transport sets no read timeout, so it waits as long as the
/// function is idle), then a `POST` of its answer to `.../{id}/response`, or
/// of its failure to `.../{id}/error`. The trace id each invocation carries
/// is set as `_X_AMZN_TRACE_ID` for its duration, as X-Ray reads it.
///
/// Two event shapes arrive, and they fail differently:
///
/// - An http event: a function url or an API Gateway HTTP api, both payload
///   format 2.0 (`rawPath`, `rawQueryString`, `headers`, `cookies`, `body`,
///   `isBase64Encoded`, `requestContext.http.method`). A non-`$default`
///   stage's prefix is stripped from the path. A client is waiting, so a
///   failed dispatch is answered with a 500 and the invocation succeeds.
/// - A [`ScheduledInvoke`], any event without `requestContext.http`. No
///   client waits: its dispatch IS the invocation, so a failure fails the
///   invocation through `/error`, or a job broken for weeks reports green in
///   every metric the schedule publishes.
#[derive(Debug, Clone)]
pub struct LambdaRuntime {
	/// `host:port` of the Runtime API.
	api: SmolStr,
}

/// One invocation as `next` answers it.
struct Invocation {
	id: SmolStr,
	trace_id: Option<SmolStr>,
	event: Value,
}

impl LambdaRuntime {
	/// The variable naming the Runtime API's `host:port`.
	pub const API_VAR: &'static str = "AWS_LAMBDA_RUNTIME_API";
	/// The Runtime API version every path names.
	const VERSION: &'static str = "2018-06-01";

	/// The Runtime API at `api` (`host:port`, or a test server's url).
	pub fn new(api: impl AsRef<str>) -> Self {
		let api = api.as_ref();
		Self {
			api: api
				.strip_prefix("http://")
				.unwrap_or(api)
				.trim_end_matches('/')
				.into(),
		}
	}

	/// The Runtime API this task names.
	pub fn from_env() -> Result<Self> {
		env_ext::var(Self::API_VAR).map(Self::new).map_err(|_| {
			bevyhow!(
				"no `{}`: the lambda backend serves only inside a Lambda task",
				Self::API_VAR
			)
		})
	}

	fn url(&self, path: &str) -> String {
		format!("http://{}/{}/runtime/{path}", self.api, Self::VERSION)
	}

	/// Serve invocations through `dispatch` until the runtime stops
	/// answering.
	pub async fn run<F, Fut>(&self, dispatch: F) -> Result
	where
		F: Fn(Request) -> Fut,
		Fut: Future<Output = Response>,
	{
		loop {
			self.serve_one(&dispatch).await?;
		}
	}

	/// Take the next invocation, dispatch it and post its outcome.
	pub async fn serve_one<F, Fut>(&self, dispatch: &F) -> Result
	where
		F: Fn(Request) -> Fut,
		Fut: Future<Output = Response>,
	{
		let invocation = self.next().await?;
		if let Some(trace_id) = &invocation.trace_id {
			unsafe { env_ext::set_var("_X_AMZN_TRACE_ID", trace_id.as_str()) }?;
		}
		let outcome = match LambdaEvent::of(invocation.event) {
			LambdaEvent::Http(event) => {
				Self::serve_http(event, dispatch).await.xmap(Ok)
			}
			LambdaEvent::Scheduled(event) => {
				Self::serve_scheduled(event, dispatch).await
			}
		};
		match outcome {
			Ok(body) => {
				self.post(
					&format!("invocation/{}/response", invocation.id),
					body,
				)
				.await
			}
			Err(err) => {
				error!("invocation {} failed: {err}", invocation.id);
				self.post_error(
					&format!("invocation/{}/error", invocation.id),
					&err,
				)
				.await
			}
		}
	}

	/// Report a failure before the first invocation.
	pub async fn init_error(&self, err: &BevyError) -> Result {
		self.post_error("init/error", err).await
	}

	async fn next(&self) -> Result<Invocation> {
		let response = Request::get(self.url("invocation/next")).send().await?;
		if !response.status().is_success() {
			bevybail!(
				"the runtime api answered `next` with {}",
				response.status()
			);
		}
		let header =
			|name: &str| response.headers().first_raw(name).map(SmolStr::new);
		let Some(id) = header("lambda-runtime-aws-request-id") else {
			bevybail!("the runtime api answered `next` without a request id");
		};
		let trace_id = header("lambda-runtime-trace-id");
		let body = response.text().await?;
		Invocation {
			id,
			trace_id,
			event: serde_json::from_str(&body)?,
		}
		.xok()
	}

	async fn post(&self, path: &str, body: Value) -> Result {
		let response = Request::post(self.url(path))
			.with_header_raw("content-type", "application/json")
			.with_body(serde_json::to_vec(&body)?)
			.send()
			.await?;
		match response.status().is_success() {
			true => Ok(()),
			false => bevybail!(
				"the runtime api answered `{path}` with {}",
				response.status()
			),
		}
	}

	async fn post_error(&self, path: &str, err: &BevyError) -> Result {
		let response = Request::post(self.url(path))
			.with_header_raw("content-type", "application/json")
			.with_header_raw("lambda-runtime-function-error-type", "Unhandled")
			.with_body(serde_json::to_vec(&json!({
				"errorMessage": err.to_string(),
				"errorType": "BeetError",
			}))?)
			.send()
			.await?;
		match response.status().is_success() {
			true => Ok(()),
			false => bevybail!(
				"the runtime api answered `{path}` with {}",
				response.status()
			),
		}
	}

	/// Dispatch an http event, answering its 2.0 response: a failure is a
	/// bare 500, its cause logged rather than sent.
	async fn serve_http<F, Fut>(event: Value, dispatch: &F) -> Value
	where
		F: Fn(Request) -> Fut,
		Fut: Future<Output = Response>,
	{
		let answered = async {
			let response = dispatch(Self::request_of(&event)?).await;
			Self::response_of(response).await
		}
		.await;
		answered.unwrap_or_else(|err| {
			error!("failed to answer a lambda http event: {err}");
			// the cause stays in the log, never in front of the client
			json!({
				"statusCode": 500,
				"headers": {"content-type": "text/plain"},
				"body": "Internal Server Error",
				"isBase64Encoded": false,
			})
		})
	}

	/// Dispatch a [`ScheduledInvoke`] and resolve the invocation with it,
	/// through the same status ladder a one-shot command exits by. The body
	/// is echoed back as the result so a run's report lands in the logs; the
	/// outcome is the status.
	async fn serve_scheduled<F, Fut>(
		event: Value,
		dispatch: &F,
	) -> Result<Value>
	where
		F: Fn(Request) -> Fut,
		Fut: Future<Output = Response>,
	{
		let invoke =
			ScheduledInvoke::from_payload(&serde_json::to_vec(&event)?)?;
		let path = invoke.path().clone();
		info!("⏱ scheduled invoke: {} {path}", invoke.method());
		let (parts, body) = dispatch(invoke.into_request()).await.into_parts();
		let body = body.into_bytes().await?;
		let text = String::from_utf8_lossy(&body);
		if parts.status_to_exit_code().is_err() {
			bevybail!(
				"the scheduled invoke of `{path}` failed with {}: {text}",
				parts.status()
			);
		}
		json!({ "path": path, "result": text }).xok()
	}

	/// The request a 2.0 http event describes.
	fn request_of(event: &Value) -> Result<Request> {
		let http = &event["requestContext"]["http"];
		let method = http["method"].as_str().unwrap_or("GET");
		let method = HttpMethod::from_str(method)?;
		let mut path = event["rawPath"].as_str().unwrap_or("/").to_string();
		// a named stage prefixes the path the routes never declared
		if let Some(stage) = event["requestContext"]["stage"].as_str()
			&& stage != "$default"
			&& let Some(rest) = path.strip_prefix(&format!("/{stage}"))
			&& (rest.is_empty() || rest.starts_with('/'))
		{
			path = match rest.is_empty() {
				true => "/".into(),
				false => rest.into(),
			};
		}
		let url = match event["rawQueryString"].as_str() {
			Some(query) if !query.is_empty() => format!("{path}?{query}"),
			_ => path,
		};
		let mut request = Request::new(method, url.as_str());
		for (name, value) in event["headers"].as_object().into_iter().flatten()
		{
			if let Some(value) = value.as_str() {
				request = request.with_header_raw(name, value);
			}
		}
		let cookies = event["cookies"]
			.as_array()
			.into_iter()
			.flatten()
			.filter_map(Value::as_str)
			.collect::<Vec<_>>();
		if !cookies.is_empty() {
			request = request.with_header_raw("cookie", &cookies.join("; "));
		}
		if let Some(ip) = http["sourceIp"]
			.as_str()
			.and_then(|ip| ip.parse::<std::net::IpAddr>().ok())
		{
			request = request.with_header_raw(
				PEER_ADDR_HEADER,
				&std::net::SocketAddr::new(ip, 0).to_string(),
			);
		}
		let body = match (
			event["body"].as_str(),
			event["isBase64Encoded"].as_bool(),
		) {
			(Some(body), Some(true)) => Bytes::from(
				base64::engine::general_purpose::STANDARD.decode(body)?,
			),
			(Some(body), _) => Bytes::from(body.to_string()),
			(None, _) => Bytes::new(),
		};
		request.with_body(body).xok()
	}

	/// A response as the 2.0 format answers it: every `set-cookie` in
	/// `cookies`, other repeated headers joined, a body that is not utf-8
	/// base64 encoded.
	async fn response_of(response: Response) -> Result<Value> {
		let (parts, body) = response.into_parts();
		let bytes = body.into_bytes().await?;
		let mut headers = serde_json::Map::new();
		let mut cookies = Vec::new();
		for (name, values) in parts.headers().iter_all() {
			match name.as_str() {
				"set-cookie" => cookies.extend(values.iter().cloned()),
				_ => {
					headers.insert(name.clone(), json!(values.join(",")));
				}
			}
		}
		let (body, is_base64) = match String::from_utf8(bytes.to_vec()) {
			Ok(text) => (text, false),
			Err(_) => (
				base64::engine::general_purpose::STANDARD.encode(&bytes),
				true,
			),
		};
		json!({
			"statusCode": parts.status().as_u16(),
			"headers": headers,
			"cookies": cookies,
			"body": body,
			"isBase64Encoded": is_base64,
		})
		.xok()
	}
}

/// The two shapes an invocation's event takes.
enum LambdaEvent {
	/// Payload format 2.0, a client waiting.
	Http(Value),
	/// Anything without `requestContext.http`, read as a
	/// [`ScheduledInvoke`].
	Scheduled(Value),
}

impl LambdaEvent {
	fn of(event: Value) -> Self {
		match event["requestContext"]["http"].is_object() {
			true => Self::Http(event),
			false => Self::Scheduled(event),
		}
	}
}

#[cfg(all(test, feature = "server", feature = "ureq"))]
mod test {
	use super::*;
	use std::sync::Arc;
	use std::sync::Mutex;

	/// A fake Runtime API serving `events` in order, keeping every post.
	async fn fake_api(
		events: Vec<Value>,
	) -> (LambdaRuntime, Arc<Mutex<Vec<(String, Value)>>>) {
		let events = Arc::new(Mutex::new(events.into_iter().enumerate()));
		let posts = Arc::new(Mutex::new(Vec::new()));
		let seen = posts.clone();
		let url = HttpServer::spawn_test(exchange_ext::handler_async(
			move |request: Request| {
				let events = events.clone();
				let seen = seen.clone();
				async move {
					let path = request.url().path_string();
					match request.method() {
						HttpMethod::Get => {
							match events.lock().unwrap().next() {
								Some((index, event)) => {
									Response::ok_text(event.to_string())
										.with_header(
											"lambda-runtime-aws-request-id",
											&format!("req-{index}"),
										)
										.with_header(
											"lambda-runtime-trace-id",
											"Root=1-abc",
										)
								}
								None => Response::from_status(StatusCode::GONE),
							}
						}
						_ => {
							let body = request.body.into_bytes().await.unwrap();
							let body = serde_json::from_slice(&body).unwrap();
							seen.lock().unwrap().push((path, body));
							Response::from_status(StatusCode::ACCEPTED)
						}
					}
				}
			},
		))
		.await;
		(LambdaRuntime::new(url), posts)
	}

	/// A route answering what it was asked, with a cookie and, for
	/// `/binary`, a body that is not utf-8.
	async fn dispatch(request: Request) -> Response {
		match request.url().path_string().as_str() {
			"/binary" => Response::ok().with_body(vec![0xff, 0xfe, 0x00]),
			"/fail" => Response::from_status(StatusCode::INTERNAL_SERVER_ERROR)
				.with_body("broke"),
			"/jobs/ok" => Response::ok_text("rolled up"),
			path => {
				let echo = json!({
					"method": request.method().to_string(),
					"path": path,
					"query": request.url().query_string(),
					"cookie": request.headers.first_raw("cookie"),
					"x-test": request.headers.first_raw("x-test"),
					"body": String::from_utf8(
						request.body.into_bytes().await.unwrap().to_vec()
					).unwrap(),
				});
				Response::ok_text(echo.to_string())
					.with_header("set-cookie", "a=1")
					.with_header("set-cookie", "b=2")
			}
		}
	}

	fn http_event(path: &str, stage: &str, body: &str, base64: bool) -> Value {
		json!({
			"version": "2.0",
			"rawPath": path,
			"rawQueryString": "q=1",
			"cookies": ["session=abc", "theme=dark"],
			"headers": {"x-test": "yes", "host": "fn.lambda-url.aws"},
			"requestContext": {
				"http": {"method": "POST", "sourceIp": "203.0.113.9"},
				"stage": stage,
			},
			"body": body,
			"isBase64Encoded": base64,
		})
	}

	/// An http event round trips: the stage stripped, headers, cookies and
	/// a base64 body in, every `set-cookie` and a base64 binary body out.
	#[beet_core::test]
	async fn round_trips_an_http_event() {
		let (runtime, posts) = fake_api(vec![
			http_event("/prod/echo", "prod", "aGVsbG8=", true),
			http_event("/binary", "$default", "", false),
		])
		.await;
		runtime.serve_one(&dispatch).await.unwrap();
		runtime.serve_one(&dispatch).await.unwrap();
		let posts = posts.lock().unwrap().clone();
		posts[0]
			.0
			.as_str()
			.xpect_eq("/2018-06-01/runtime/invocation/req-0/response");
		let answer = &posts[0].1;
		answer["statusCode"].xpect_eq(json!(200));
		answer["cookies"].xpect_eq(json!(["a=1", "b=2"]));
		serde_json::from_str::<Value>(answer["body"].as_str().unwrap())
			.unwrap()
			.xpect_eq(json!({
				"method": "Post",
				"path": "/echo",
				"query": "q=1",
				"cookie": "session=abc; theme=dark",
				"x-test": "yes",
				"body": "hello",
			}));
		posts[1].1["isBase64Encoded"].xpect_eq(json!(true));
		posts[1].1["body"].xpect_eq(json!("//4A"));
		env_ext::var("_X_AMZN_TRACE_ID")
			.unwrap()
			.as_str()
			.xpect_eq("Root=1-abc");
	}

	/// A scheduled invoke that succeeds answers its report.
	#[beet_core::test]
	async fn answers_a_scheduled_success() {
		let (runtime, posts) = fake_api(vec![
			serde_json::to_value(ScheduledInvoke::new("jobs/ok")).unwrap(),
		])
		.await;
		runtime.serve_one(&dispatch).await.unwrap();
		let posts = posts.lock().unwrap().clone();
		posts[0]
			.0
			.as_str()
			.xpect_eq("/2018-06-01/runtime/invocation/req-0/response");
		posts[0]
			.1
			.xpect_eq(json!({"path": "jobs/ok", "result": "rolled up"}));
	}

	/// A scheduled invoke that fails fails the invocation through `/error`.
	#[beet_core::test]
	async fn fails_a_scheduled_failure() {
		let (runtime, posts) = fake_api(vec![
			serde_json::to_value(ScheduledInvoke::new("fail")).unwrap(),
		])
		.await;
		runtime.serve_one(&dispatch).await.unwrap();
		let posts = posts.lock().unwrap().clone();
		posts[0]
			.0
			.as_str()
			.xpect_eq("/2018-06-01/runtime/invocation/req-0/error");
		posts[0].1["errorMessage"]
			.as_str()
			.unwrap()
			.xpect_contains("the scheduled invoke of `fail` failed");
	}

	/// A failure before the first invocation reaches `init/error`.
	#[beet_core::test]
	async fn reports_an_init_failure() {
		let (runtime, posts) = fake_api(vec![]).await;
		runtime
			.init_error(&bevyhow!("no dispatch host"))
			.await
			.unwrap();
		let posts = posts.lock().unwrap().clone();
		posts[0]
			.0
			.as_str()
			.xpect_eq("/2018-06-01/runtime/init/error");
		posts[0].1["errorMessage"]
			.as_str()
			.unwrap()
			.xpect_contains("no dispatch host");
	}
}
