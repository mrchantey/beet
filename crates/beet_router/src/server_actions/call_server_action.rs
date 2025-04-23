use crate::prelude::*;
use once_cell::sync::Lazy;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::sync::Mutex;
use sweet::net::exports::reqwest::RequestBuilder;
use sweet::prelude::*;

static SERVER_URL: Lazy<Mutex<RoutePath>> =
	Lazy::new(|| Mutex::new("http://127.0.0.1:3000".into()));

pub struct CallServerAction;

impl CallServerAction {
	pub fn get_server_url() -> RoutePath { SERVER_URL.lock().unwrap().clone() }
	pub fn set_server_url(url: RoutePath) { *SERVER_URL.lock().unwrap() = url; }

	/// Makes a HTTP request to a server action.
	/// Automatically uses the correct request style based on the HTTP method:
	/// - Bodyless methods (GET, HEAD, DELETE, OPTIONS, CONNECT, TRACE) send data as query parameters
	/// - Methods with body (POST, PUT, PATCH) send data in the request body
	pub async fn request<T: Serialize, O: DeserializeOwned>(
		route_info: RouteInfo,
		value: T,
	) -> Result<O, ServerActionError> {
		if route_info.method.has_body() {
			Self::request_with_body(route_info, value).await
		} else {
			Self::request_with_query(route_info, value).await
		}
	}
	//// Makes a HTTP request to a server action without any data.
	pub async fn request_no_data<O: DeserializeOwned>(
		route_info: RouteInfo,
	) -> Result<O, ServerActionError> {
		let url = SERVER_URL.lock().unwrap().join(&route_info.path);
		let method = route_info.method.into();
		Self::send(
			route_info,
			ReqwestClient::client().request(method, url.to_string()),
		)
		.await
	}

	/// Internal function to make a request with data in the query parameters,
	/// for deserilization by [`JsonQuery`].
	/// Used by GET, HEAD, DELETE, OPTIONS, CONNECT, TRACE methods.
	async fn request_with_query<T: Serialize, O: DeserializeOwned>(
		route_info: RouteInfo,
		value: T,
	) -> Result<O, ServerActionError> {
		let value = serde_json::to_string(&value)
			.map_err(|e| ServerActionError::Serialize(route_info.clone(), e))?;

		let url = SERVER_URL.lock().unwrap().join(&route_info.path);
		let method = route_info.method.into();
		Self::send(
			route_info,
			ReqwestClient::client()
				.request(method, url.to_string())
				.query(&[("data", value)]),
		)
		.await
	}

	/// Internal function to make a request with data in the request body.
	/// Used by POST, PUT, PATCH methods.
	async fn request_with_body<T: Serialize, O: DeserializeOwned>(
		route_info: RouteInfo,
		value: T,
	) -> Result<O, ServerActionError> {
		let value = serde_json::to_string(&value)
			.map_err(|e| ServerActionError::Serialize(route_info.clone(), e))?;

		let url = SERVER_URL.lock().unwrap().join(&route_info.path);
		let method = route_info.method.into();
		Self::send(
			route_info,
			ReqwestClient::client()
				.request(method, url.to_string())
				.header("Content-Type", "application/json")
				.body(value),
		)
		.await
	}


	async fn send<O: DeserializeOwned>(
		route_info: RouteInfo,
		req: RequestBuilder,
	) -> Result<O, ServerActionError> {
		let res = req
			.send()
			.await
			.map_err(|e| ServerActionError::Request(route_info.clone(), e))?;
		let status = res.status();

		// this will always either be of type O or E
		let bytes = res.bytes().await.map_err(|e| {
			ServerActionError::ResponseBody(route_info.clone(), e)
		})?;

		if status.is_success() {
			serde_json::from_slice(&bytes)
				.map_err(|e| ServerActionError::Deserialize(route_info, e))
		} else if status.is_client_error() {
			println!("Error: {status} {bytes:?}");
			Err(ServerActionError::ClientError(
				route_info,
				status,
				String::from_utf8_lossy(&bytes).to_string(),
			))
		} else {
			Err(ServerActionError::ServerError(route_info, status))
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

	async fn add_handler_get(
		JsonQuery(params): JsonQuery<(i32, i32)>,
	) -> Json<i32> {
		Json(params.0 + params.1)
	}

	async fn add_handler_post(Json(params): Json<(i32, i32)>) -> Json<i32> {
		Json(params.0 + params.1)
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
				.route("/add", get(add_handler_get))
				.route("/add", post(add_handler_post)),
		)
		.await;
		test_get().await;
		test_post().await;
		rejects().await;
	}
	async fn test_get() {
		expect(
			CallServerAction::request::<_, i32>(
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
			CallServerAction::request::<_, i32>(
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
			CallServerAction::request::<_, i32>(
				RouteInfo::new("/add", HttpMethod::Post),
				7,
			)
			.await
			.unwrap_err()
			.to_string(),
		)
		.to_be("fooar");
	}
}
