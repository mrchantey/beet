use crate::prelude::*;
use beet_core::prelude::*;
use bevy::prelude::*;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::sync::LazyLock;
use std::sync::Mutex;

/// the url for the server.
/// On native builds this defaults to `http://127.0.0.1:3000`.
/// On wasm builds this is set to the current origin.
static SERVER_URL: LazyLock<Mutex<RoutePath>> = LazyLock::new(|| {
	#[cfg(not(target_arch = "wasm32"))]
	let path = "http://127.0.0.1:3000";
	#[cfg(target_arch = "wasm32")]
	let path = web_sys::window()
		.and_then(|w| w.location().origin().ok())
		.unwrap();
	Mutex::new(path.into())
});

pub struct CallServerAction;

impl CallServerAction {
	pub fn get_server_url() -> RoutePath { SERVER_URL.lock().unwrap().clone() }
	pub fn set_server_url(url: RoutePath) { *SERVER_URL.lock().unwrap() = url; }
	/// on wasm we assume the same domain and dont prefix SERVER_URL.
	fn create_url(route_info: &RouteInfo) -> String {
		format!("{}{}", CallServerAction::get_server_url(), route_info.path)
	}
	/// Makes a HTTP request to a server action.
	/// Automatically uses the correct request style based on the HTTP method:
	/// - Bodyless methods (GET, HEAD, DELETE, OPTIONS, CONNECT, TRACE) send data as query parameters
	/// - Methods with body (POST, PUT, PATCH) send data in the request body
	pub async fn request<O: DeserializeOwned, E: DeserializeOwned>(
		route_info: RouteInfo,
		value: impl Serialize,
	) -> ServerActionResult<O, E> {
		if route_info.method.has_body() {
			Self::request_with_body(route_info, value).await
		} else {
			Self::request_with_query(route_info, value).await
		}
		.map_err(ServerActionError::from)
	}
	//// Makes a HTTP request to a server action without any data.
	pub async fn request_no_data<O: DeserializeOwned, E: DeserializeOwned>(
		route_info: RouteInfo,
	) -> ServerActionResult<O, E> {
		let req =
			Request::new(route_info.method, Self::create_url(&route_info));
		Self::send(req).await.map_err(ServerActionError::from)
	}

	/// Internal function to make a request with data in the query parameters.
	/// This will be first serialized as json and then encoded as a query parameter
	/// for deserilaization by [`JsonQuery`].
	/// Used by GET, HEAD, DELETE, OPTIONS, CONNECT, TRACE methods.
	async fn request_with_query<O: DeserializeOwned, E: DeserializeOwned>(
		route_info: RouteInfo,
		value: impl Serialize,
	) -> ServerActionResult<O, E> {
		let payload = JsonQueryParams::to_query_string(&value)
			.map_err(ServerActionError::from_opaque)?;
		let req =
			Request::new(route_info.method, Self::create_url(&route_info))
				.with_query_string(&payload)
				.map_err(ServerActionError::from_opaque)?;
		Self::send(req).await
	}

	/// Internal function to make a request with data in the request body.
	/// Used by POST, PUT, PATCH methods.
	async fn request_with_body<O: DeserializeOwned, E: DeserializeOwned>(
		route_info: RouteInfo,
		value: impl Serialize,
	) -> ServerActionResult<O, E> {
		let req =
			Request::new(route_info.method, Self::create_url(&route_info))
				.with_json_body(&value)
				.map_err(ServerActionError::from_opaque)?;
		Self::send(req).await
	}


	async fn send<O: DeserializeOwned, E: DeserializeOwned>(
		req: Request,
	) -> ServerActionResult<O, E> {
		let res = req.send().await.map_err(ServerActionError::from_opaque)?;
		let status = res.status();

		if status.is_success() {
			res.json().map_err(ServerActionError::from_opaque)
			// if not success, try to deserialize the error
			// using the actions Error type
		} else if let Ok(err) = serde_json::from_slice::<E>(&res.body) {
			Err(ServerActionError::ActionError(err))
		} else {
			Err(ServerActionError::HttpError::<E>(res.into()))
		}
	}
}


#[cfg(test)]
#[cfg(all(feature = "axum", not(target_arch = "wasm32")))]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;
	use tokio::net::TcpListener;
	use tokio::task::JoinHandle;

	fn add_via_get(In(params): In<(i32, i32)>) -> i32 { params.0 + params.1 }
	fn add_via_post(In(params): In<(i32, i32)>) -> i32 { params.0 + params.1 }
	fn increment_if_positive(In(params): In<i32>) -> Result<i32, String> {
		if params > 0 {
			Ok(params + 1)
		} else {
			Err(format!("expected positive number, received {params}"))
		}
	}

	// fn parse_err

	#[must_use]
	async fn serve(router: axum::Router) -> JoinHandle<()> {
		// random port assigned
		let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
		let addr = listener.local_addr().unwrap();
		CallServerAction::set_server_url(RoutePath::new(format!(
			"http://{}",
			addr
		)));

		// Start the server in a separate task, dropped on exit
		tokio::spawn(async move {
			axum::serve(listener, router).await.unwrap();
		})
	}
	// only a single entry because set_server_url is static
	#[sweet::test]
	async fn works() {
		let mut app = App::new();
		app.add_plugins(RouterPlugin::default());
		app.world_mut().spawn(children![
			(
				RouteFilter::new("/add").with_method(HttpMethod::Get),
				RouteHandler::action(
					HttpMethod::Get,
					add_via_get.pipe(Json::pipe)
				)
			),
			(
				RouteFilter::new("/add").with_method(HttpMethod::Post),
				RouteHandler::action(
					HttpMethod::Post,
					add_via_post.pipe(Json::pipe)
				)
			),
			(
				RouteFilter::new("/increment_if_positive")
					.with_method(HttpMethod::Get),
				RouteHandler::action(
					HttpMethod::Get,
					increment_if_positive.pipe(JsonResult::pipe)
				)
			),
		]);

		let router =
			AxumRunner::from_world(app.world_mut(), Default::default());
		let _handle = serve(router).await;
		test_get().await;
		test_post().await;
		test_result().await;
	}
	async fn test_get() {
		expect(
			CallServerAction::request::<i32, ()>(
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
			CallServerAction::request::<i32, ()>(
				RouteInfo::new("/add", HttpMethod::Post),
				(10, 7),
			)
			.await
			.unwrap(),
		)
		.to_be(17);
	}
	async fn test_result() {
		expect(
			CallServerAction::request::<i32, String>(
				RouteInfo::new("/increment_if_positive", HttpMethod::Get),
				7,
			)
			.await
			.unwrap(),
		)
		.to_be(8);
		expect(
			CallServerAction::request::<i32, String>(
				RouteInfo::new("/increment_if_positive", HttpMethod::Get),
				-7,
			)
			.await
			.unwrap_err()
			.to_string(),
		)
		.to_be("expected positive number, received -7");
	}
}
