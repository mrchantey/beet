todo!("convert from axum to beet");

/// Append no-cache headers to a response
pub fn append_no_cache_headers(val: impl IntoResponse) -> Response {
	let mut response = val.into_response();
	let headers = response.headers_mut();
	headers.insert(
		header::CACHE_CONTROL,
		HeaderValue::from_static("no-cache, no-store, must-revalidate"),
	);
	headers.insert(header::PRAGMA, HeaderValue::from_static("no-cache"));
	headers.insert(header::EXPIRES, HeaderValue::from_static("0"));
	// do something with `response`...

	response
}


#[derive(Debug, Default, Clone)]
pub struct CorsState {
	pub allow_any_origin: bool,
	allowed_origins: Vec<HeaderValue>,
}

impl CorsState {
	pub const ANY_ORIGIN: HeaderValue = HeaderValue::from_static("*");

	pub fn new(
		allow_any_origin: bool,
		allowed_origins: Vec<&'static str>,
	) -> Self {
		Self {
			allow_any_origin,
			allowed_origins: allowed_origins
				.into_iter()
				.map(|s| HeaderValue::from_static(s))
				.collect(),
		}
	}

	pub fn new_with_env(allowed_origins: Vec<&'static str>) -> Self {
		let allow_any_origin = match ApiEnvironment::get() {
			ApiEnvironment::Local => true,
			ApiEnvironment::Staging => false,
			ApiEnvironment::Prod => false,
		};

		Self::new(allow_any_origin, allowed_origins)
	}

	pub fn origin_allowed(&self, origin: &HeaderValue) -> bool {
		self.allow_any_origin || self.allowed_origins.contains(origin)
	}
}



/// TODO handle unwrap
/// Why are we allowing cors?
pub async fn cors(
	State(state): State<CorsState>,
	req: Request,
	// State(server_settings): State<ServerSettings>,
	next: Next,
) -> Response {
	let origin = req.headers().get(header::ORIGIN).cloned();
	let origin = match (state.allow_any_origin, origin) {
		(true, None) => CorsState::ANY_ORIGIN,
		(true, Some(origin)) => origin,
		(false, None) => {
			return (StatusCode::BAD_REQUEST, "Origin header not found")
				.into_response();
		}
		(false, Some(origin)) => origin,
	};

	if !state.origin_allowed(&origin) {
		return (StatusCode::FORBIDDEN, "Origin not allowed").into_response();
	}

	let is_options = req.method() == Method::OPTIONS;
	let mut res = if is_options {
		let mut res = "".into_response();
		res.headers_mut().insert(
			header::ACCESS_CONTROL_MAX_AGE,
			HeaderValue::from_static("60"), //60 seconds
		);
		res.headers_mut().insert(
			header::ACCESS_CONTROL_ALLOW_HEADERS,
			HeaderValue::from_static("content-type"),
		);
		res
	} else {
		next.run(req).await
	};

	let headers = res.headers_mut();

	headers.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, origin);

	// headers.insert(
	// 	header::ACCESS_CONTROL_ALLOW_CREDENTIALS,
	// 	HeaderValue::from_static("true"),
	// );

	// headers.insert(
	// 	header::ACCESS_CONTROL_ALLOW_METHODS,
	// 	HeaderValue::from_static("*"),
	// );

	res
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use axum::Router;
	use axum::middleware;
	use axum::response::IntoResponse;
	use axum::routing::get;
	use beet_net::exports::http::*;
	use beet_core::prelude::*;
	use tower::util::ServiceExt;

	async fn handler() -> impl IntoResponse { StatusCode::OK }

	fn req(origin: &str) -> Request<String> {
		Request::builder()
			.uri("/")
			.method(Method::GET)
			.header(header::ORIGIN, origin)
			.body(Default::default())
			.unwrap()
	}

	fn router(state: CorsState) -> Router {
		Router::new()
			.route("/", get(handler))
			.layer(middleware::from_fn_with_state(state, cors))
	}


	#[sweet::test]
	async fn works() {
		let router = router(CorsState::new(false, vec!["https://allowed.com"]));
		let req = req("https://allowed.com");

		let res = router.oneshot(req).await.unwrap();

		res.status().xpect_eq(StatusCode::OK);
		res.headers()
			.get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
			.unwrap()
			.xpect_eq("https://allowed.com");
	}
	#[sweet::test]
	async fn allows_local_any() {
		let router = router(CorsState::new_with_env(vec![]));

		let req = req("https://blocked.com");

		let res = router.oneshot(req).await.unwrap();

		res.status().xpect_eq(StatusCode::OK);
		res.headers()
			.get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
			.unwrap()
			.xpect_eq("https://blocked.com");
	}
	#[sweet::test]
	async fn blocks() {
		let router = router(CorsState::new(false, vec![]));

		let req = req("https://blocked.com");

		let res = router.oneshot(req).await.unwrap();

		res.status().xpect_eq(StatusCode::FORBIDDEN);
		res.headers()
			.get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
			.xpect_none();
	}
	#[sweet::test]
	async fn blocks_no_req_header() {
		let router = router(CorsState::new(false, vec![]));

		let req = Request::builder()
			.uri("/")
			.method(Method::GET)
			.body(String::default())
			.unwrap();

		let res = router.oneshot(req).await.unwrap();

		res.status().xpect_eq(StatusCode::BAD_REQUEST);
		res.headers()
			.get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
			.xpect_none();
	}
}
