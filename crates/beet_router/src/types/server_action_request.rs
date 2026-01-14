use beet_core::prelude::*;
use beet_net::prelude::*;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::sync::LazyLock;
use std::sync::Mutex;

/// the url for the server.
/// On native builds this defaults to `http://127.0.0.1:3000`.
/// On wasm builds this is set to the current origin.
static SERVER_URL: LazyLock<Mutex<Url>> = LazyLock::new(|| {
	#[cfg(not(target_arch = "wasm32"))]
	let path = DEFAULT_SERVER_LOCAL_URL;
	#[cfg(target_arch = "wasm32")]
	let path = beet_core::exports::web_sys::window()
		.and_then(|w| w.location().origin().ok())
		.unwrap();
	Mutex::new(Url::parse(&path).unwrap())
});


pub struct ServerActionRequest<Req = ()> {
	/// The base url of the server, defaults to [`ServerActionRequest::get_server_url`].
	pub base_url: Url,
	/// The path to the action, appended to the base url.
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
			base_url: Self::get_server_url(),
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
			base_url: Self::get_server_url(),
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
			base_url: self.base_url,
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

	pub fn with_base_url(mut self, url: Url) -> Self {
		self.base_url = url;
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
			// Route paths already have a leading slash
			self.base_url.to_string().trim_end_matches("/"),
			self.route_path
		);
		let req = Request::new(self.method, url);
		match (self.method.has_body(), self.req_body) {
			(_, None) => req,
			(true, Some(body)) => req.with_json_body(&body)?,
			(false, Some(body)) => {
				let payload = JsonQueryParams::to_query_string(&body)?;
				req.with_query_string(&payload)
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
			status if status.is_ok() => {
				res.json::<JsonResult<Res, Err>>().await?.result.xok()
			}
			// failed request
			_ => Err(res.into_error().await.into()),
		}
	}
}

#[cfg(test)]
#[cfg(all(not(target_arch = "wasm32"), feature = "server"))]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_flow::prelude::*;
	use beet_net::prelude::*;


	#[test]
	fn req_path() {
		ServerActionRequest::new(HttpMethod::Get, "/foo")
			.into_request()
			.unwrap()
			.path()
			.xpect_eq(vec!["foo".to_string()]);
	}

	fn add_via_get(In(params): In<(i32, i32)>) -> i32 { params.0 + params.1 }
	fn add_via_post(In(params): In<(i32, i32)>) -> i32 { params.0 + params.1 }
	fn increment_if_positive(In(params): In<i32>) -> Result<i32, String> {
		if params > 0 {
			Ok(params + 1)
		} else {
			Err(format!("expected positive number, received {params}"))
		}
	}

	// only a single entry because set_server_url is static
	#[sweet::test]
	async fn works() {
		let server = HttpServer::new_test();
		let url = server.local_url();
		let url = Url::parse(&url).unwrap();
		let _handle = std::thread::spawn(move || {
			let mut app = App::new();
			app.add_plugins((MinimalPlugins, RouterPlugin));
			let world = app.world_mut();
			// let mut world = ServerPlugin::with_server(server).into_world();
			world.init_resource::<RenderMode>();
			world.spawn((
				server,
				ExchangeSpawner::new_flow(|| {
					(InfallibleSequence, children![
						ServerAction::new(
							HttpMethod::Get,
							add_via_get.pipe(Json::pipe)
						)
						.with_path("add"),
						ServerAction::new(
							HttpMethod::Post,
							add_via_post.pipe(Json::pipe)
						)
						.with_path("add"),
						ServerAction::new(
							HttpMethod::Get,
							increment_if_positive.pipe(JsonResult::pipe)
						)
						.with_path("increment_if_positive")
					])
				}),
			));
			app.run();
		});
		time_ext::sleep_millis(10).await;

		test_get(url.clone()).await;
		test_post(url.clone()).await;
		test_result(url.clone()).await;
	}
	async fn test_get(url: Url) {
		ServerActionRequest::new(HttpMethod::Get, "/add")
			.with_base_url(url)
			.with_body((5, 3))
			.send::<i32>()
			.await
			.unwrap()
			.xpect_eq(8);
	}
	async fn test_post(url: Url) {
		ServerActionRequest::new(HttpMethod::Post, "/add")
			.with_base_url(url)
			.with_body((10, 7))
			.send::<i32>()
			.await
			.unwrap()
			.xpect_eq(17);
	}
	async fn test_result(url: Url) {
		ServerActionRequest::new(HttpMethod::Get, "/increment_if_positive")
			.with_base_url(url.clone())
			.with_body(7)
			.send_fallible::<i32, String>()
			.await
			.unwrap()
			.unwrap()
			.xpect_eq(8);

		ServerActionRequest::new(HttpMethod::Get, "/increment_if_positive")
			.with_base_url(url)
			.with_body(-7)
			.send_fallible::<i32, String>()
			.await
			.unwrap()
			.unwrap_err()
			.to_string()
			.xpect_eq("expected positive number, received -7");
	}
}
