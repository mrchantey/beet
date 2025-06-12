use crate::prelude::*;
use once_cell::sync::Lazy;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::sync::Mutex;
use beet_net::prelude::*;
use sweet::prelude::*;

/// the url for the server.
/// On native builds this defaults to `http://127.0.0.1:3000`.
/// On wasm builds this is set to the current origin.
#[cfg(not(target_arch = "wasm32"))]
static SERVER_URL: Lazy<Mutex<RoutePath>> =
	Lazy::new(|| Mutex::new("http://127.0.0.1:3000".into()));

#[cfg(target_arch = "wasm32")]
static SERVER_URL: Lazy<Mutex<RoutePath>> = Lazy::new(|| {
	Mutex::new(
		web_sys::window()
			.and_then(|w| w.location().origin().ok())
			.unwrap()
			.into(),
	)
});

pub struct CallServerAction;

impl CallServerAction {
	pub fn get_server_url() -> RoutePath { SERVER_URL.lock().unwrap().clone() }
	pub fn set_server_url(url: RoutePath) { *SERVER_URL.lock().unwrap() = url; }
	/// on wasm we assume the same domain and dont prefix SERVER_URL.
	fn create_url(route_info: &RouteInfo) -> String {
		format!("{}{}", Self::get_server_url(), route_info.path)
	}


	/// Makes a HTTP request to a server action.
	/// Automatically uses the correct request style based on the HTTP method:
	/// - Bodyless methods (GET, HEAD, DELETE, OPTIONS, CONNECT, TRACE) send data as query parameters
	/// - Methods with body (POST, PUT, PATCH) send data in the request body
	pub async fn request<
		T: Serialize,
		O: DeserializeOwned,
		E: DeserializeOwned,
	>(
		route_info: RouteInfo,
		value: T,
	) -> ServerActionResult<O, E> {
		if route_info.method.has_body() {
			Self::request_with_body(route_info, value).await
		} else {
			Self::request_with_query(route_info, value).await
		}
	}
	//// Makes a HTTP request to a server action without any data.
	pub async fn request_no_data<O: DeserializeOwned, E: DeserializeOwned>(
		route_info: RouteInfo,
	) -> ServerActionResult<O, E> {
		let req = Request::new(Self::create_url(&route_info))
			.method(route_info.method);
		Self::send(route_info, req).await
	}

	/// Internal function to make a request with data in the query parameters.
	/// This will be first serialized as json and then encoded as a query parameter
	/// for deserilaization by [`JsonQuery`].
	/// Used by GET, HEAD, DELETE, OPTIONS, CONNECT, TRACE methods.
	async fn request_with_query<
		T: Serialize,
		O: DeserializeOwned,
		E: DeserializeOwned,
	>(
		route_info: RouteInfo,
		value: T,
	) -> ServerActionResult<O, E> {
		let payload = serde_json::to_string(&value)
			.map_err(|err| cross_fetch::Error::serialization(err))?;

		let req = Request::new(Self::create_url(&route_info))
			.method(route_info.method)
			.query(&[("data", payload)])?;
		Self::send(route_info, req).await
	}

	/// Internal function to make a request with data in the request body.
	/// Used by POST, PUT, PATCH methods.
	async fn request_with_body<
		T: Serialize,
		O: DeserializeOwned,
		E: DeserializeOwned,
	>(
		route_info: RouteInfo,
		value: T,
	) -> ServerActionResult<O, E> {
		let req = Request::new(Self::create_url(&route_info))
			.method(route_info.method)
			.body(value)?;
		Self::send(route_info, req).await
	}


	async fn send<O: DeserializeOwned, E: DeserializeOwned>(
		_route_info: RouteInfo,
		req: Request,
	) -> ServerActionResult<O, E> {
		let res = req.send().await?;
		let status = res.status_code();

		let body_bytes = res.bytes().await?;

		if status.is_success() {
			serde_json::from_slice::<O>(&body_bytes)
				.map_err(|err| cross_fetch::Error::serialization(err))?
				.xok()
		} else if let Ok(err) = serde_json::from_slice(&body_bytes) {
			Err(ServerActionError::ActionError(err))
		} else {
			let str = String::from_utf8_lossy(&body_bytes).to_string();
			Err(ServerActionError::UnparsedError(status, str))
		}
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use axum::Json;
	use axum::Router;
	use axum::routing::get;
	use axum::routing::post;
	use sweet::prelude::*;
	use tokio::net::TcpListener;
	use tokio::spawn;
	use tokio::task::JoinHandle;

	async fn add_via_get(
		JsonQuery(params): JsonQuery<(i32, i32)>,
	) -> Json<i32> {
		Json(params.0 + params.1)
	}

	async fn add_via_post(Json(params): Json<(i32, i32)>) -> Json<i32> {
		Json(params.0 + params.1)
	}

	fn check(val: i32) -> anyhow::Result<()> {
		if val > 0 {
			Ok(())
		} else {
			anyhow::bail!("expected positive number, received {val}")
		}
	}

	async fn reject_neg(Json(params): Json<i32>) -> Result<(), ActionError> {
		check(params).into_action_result()
	}

	#[must_use]
	async fn serve(router: Router) -> JoinHandle<()> {
		// random port assigned
		let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
		let addr = listener.local_addr().unwrap();
		CallServerAction::set_server_url(RoutePath::new(format!(
			"http://{}",
			addr
		)));

		// Start the server in a separate task, dropped on exit
		spawn(async move {
			axum::serve(listener, router).await.unwrap();
		})
	}

	// only a single entry because set_server_url is static
	#[sweet::test]
	async fn works() {
		let _server = serve(
			Router::new()
				.route("/add", get(add_via_get))
				.route("/add", post(add_via_post))
				.route("/reject_neg", post(reject_neg)),
		)
		.await;
		test_get().await;
		test_post().await;
		rejects().await;
	}
	async fn test_get() {
		expect(
			CallServerAction::request::<_, i32, ()>(
				RouteInfo::new("/add", HttpMethod::Get),
				(5, 3),
			)
			.await
			.unwrap(),
		)
		.to_be(8);
	}
	async fn test_post() {
		expect(
			CallServerAction::request::<_, i32, ()>(
				RouteInfo::new("/add", HttpMethod::Post),
				(10, 7),
			)
			.await
			.unwrap(),
		)
		.to_be(17);
	}
	async fn rejects() {
		expect(
			CallServerAction::request::<_, i32, String>(
				RouteInfo::new("/reject_neg", HttpMethod::Post),
				-7,
			)
			.await
			.unwrap_err()
			.to_string(),
		)
		.to_contain("expected positive number, received -7");
	}
}
