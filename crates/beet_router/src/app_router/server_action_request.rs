use beet_net::prelude::*;
use beet_core::prelude::*;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::sync::LazyLock;
use std::sync::Mutex;

/// the url for the server.
/// On native builds this defaults to `http://127.0.0.1:3000`.
/// On wasm builds this is set to the current origin.
static SERVER_URL: LazyLock<Mutex<Url>> = LazyLock::new(|| {
	#[cfg(not(target_arch = "wasm32"))]
	let path = "http://127.0.0.1:3000";
	#[cfg(target_arch = "wasm32")]
	let path = beet_core::exports::web_sys::window()
		.and_then(|w| w.location().origin().ok())
		.unwrap();
	Mutex::new(Url::parse(&path).unwrap())
});


pub struct ServerActionRequest<Req = ()> {
	pub route_path: RoutePath,
	pub method: HttpMethod,
	/// The status code to check when calling [`Self::send_fallible`],
	/// determining if the body should be treated as the [`Err`] type.
	pub error_status: StatusCode,
	pub req_body: Option<Req>,
}

impl Default for ServerActionRequest {
	fn default() -> Self {
		Self {
			method: HttpMethod::Get,
			error_status: JsonResult::DEFAULT_ERR_STATUS,
			route_path: RoutePath::default(),
			req_body: None,
		}
	}
}

impl ServerActionRequest<()> {
	pub fn new(method: HttpMethod, route_path: impl Into<RoutePath>) -> Self {
		Self {
			method,
			error_status: JsonResult::DEFAULT_ERR_STATUS,
			route_path: route_path.into(),
			req_body: None,
		}
	}
}

impl ServerActionRequest {
	pub fn get_server_url() -> Url { SERVER_URL.lock().unwrap().clone() }
	pub fn set_server_url(url: Url) { *SERVER_URL.lock().unwrap() = url; }
}

impl<Req> ServerActionRequest<Req> {
	pub fn with_body<Req2>(self, body: Req2) -> ServerActionRequest<Req2> {
		ServerActionRequest {
			route_path: self.route_path,
			method: self.method,
			error_status: self.error_status,
			req_body: Some(body),
		}
	}
	pub fn with_error_status(mut self, status: StatusCode) -> Self {
		self.error_status = status;
		self
	}
}

impl<Req> ServerActionRequest<Req>
where
	Req: Serialize,
{
	pub fn into_request(self) -> Result<Request> {
		let url = format!(
			"{}{}",
			ServerActionRequest::get_server_url(),
			self.route_path
		);
		let req = Request::new(self.method, url);
		match (self.method.has_body(), self.req_body) {
			(_, None) => req,
			(true, Some(body)) => req.with_json_body(&body)?,
			(false, Some(body)) => {
				let payload = JsonQueryParams::to_query_string(&body)?;
				req.with_query_string(&payload)?
			}
		}
		.xok()
	}

	pub async fn send<Res>(self) -> Result<Res>
	where
		Res: DeserializeOwned,
	{
		self.into_request()?
			.send()
			.await?
			.into_result()
			.await?
			.json()
			.await
	}
	pub async fn send_fallible<Res, Err>(self) -> Result<Result<Res, Err>>
	where
		Res: DeserializeOwned,
		Err: DeserializeOwned,
	{
		let error_status = self.error_status;
		let res = self.into_request()?.send().await?;
		match res.status() {
			// succesfull request, handler failed
			status if status == error_status => {
				let err = res.json().await?;
				Ok(Err(err))
			}
			// successful request, handler succeeded
			status if status.is_success() => {
				let ok = res.json().await?;
				Ok(Ok(ok))
			}
			// failed request
			_ => Err(res.into_error().await.into()),
		}
	}
}

#[cfg(test)]
#[cfg(all(feature = "axum", not(target_arch = "wasm32")))]
mod test {
	use crate::prelude::*;
	use beet_net::prelude::*;
	use beet_core::prelude::*;
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
		ServerActionRequest::set_server_url(
			Url::parse(&format!("http://{}", addr)).unwrap(),
		);

		// Start the server in a separate task, dropped on exit
		tokio::spawn(async move {
			axum::serve(listener, router).await.unwrap();
		})
	}
	// only a single entry because set_server_url is static
	#[sweet::test]
	async fn works() {
		let mut world = World::new();
		world.init_resource::<RenderMode>();
		world.insert_resource(Router::new_bundle(|| {
			children![
				(
					PathFilter::new("/add"),
					action_endpoint(
						HttpMethod::Get,
						add_via_get.pipe(Json::pipe)
					)
				),
				(
					PathFilter::new("/add"),
					action_endpoint(
						HttpMethod::Post,
						add_via_post.pipe(Json::pipe)
					)
				),
				(
					PathFilter::new("/increment_if_positive"),
					action_endpoint(
						HttpMethod::Get,
						increment_if_positive.pipe(JsonResult::pipe)
					)
				),
			]
		}));
		let router = AxumRunner::router(&mut world);
		let _handle = serve(router).await;
		test_get().await;
		test_post().await;
		test_result().await;
	}
	async fn test_get() {
		ServerActionRequest::new(HttpMethod::Get, "/add")
			.with_body((5, 3))
			.send::<i32>()
			.await
			.unwrap()
			.xpect_eq(8);
	}
	async fn test_post() {
		ServerActionRequest::new(HttpMethod::Post, "/add")
			.with_body((10, 7))
			.send::<i32>()
			.await
			.unwrap()
			.xpect_eq(17);
	}
	async fn test_result() {
		ServerActionRequest::new(HttpMethod::Get, "/increment_if_positive")
			.with_body(7)
			.send_fallible::<i32, String>()
			.await
			.unwrap()
			.unwrap()
			.xpect_eq(8);

		ServerActionRequest::new(HttpMethod::Get, "/increment_if_positive")
			.with_body(-7)
			.send_fallible::<i32, String>()
			.await
			.unwrap()
			.unwrap_err()
			.to_string()
			.xpect_eq("expected positive number, received -7");
	}
}
