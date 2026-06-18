//! Client-side support for invoking server actions.
//!
//! Generated client-action callers build their requests via
//! [`server_action_request`], which prepends the globally configured server URL
//! to the action path. The default is the local dev server on native and the
//! current page origin on wasm; override it with [`set_server_url`].

use beet_core::prelude::*;
use beet_net::prelude::*;
use bevy::platform::sync::Mutex;
use bevy::platform::sync::OnceLock;

/// The base URL prepended to server-action paths. This is transport config,
/// not world state, so it lives in a global rather than a resource (mirroring
/// `beet_net`'s `set_http_client` hook).
static SERVER_URL: OnceLock<Mutex<Url>> = OnceLock::new();

fn server_url_cell() -> &'static Mutex<Url> {
	SERVER_URL.get_or_init(|| {
		#[cfg(not(target_arch = "wasm32"))]
		let raw = DEFAULT_SERVER_LOCAL_URL.to_string();
		#[cfg(target_arch = "wasm32")]
		let raw = beet_core::exports::web_sys::window()
			.and_then(|window| window.location().origin().ok())
			.unwrap_or_else(|| DEFAULT_SERVER_LOCAL_URL.to_string());
		Mutex::new(Url::parse(raw))
	})
}

/// Returns the currently configured server URL for client actions.
pub fn server_url() -> Url { server_url_cell().lock().unwrap().clone() }

/// Sets the server URL used by all subsequent client-action calls.
pub fn set_server_url(url: impl Into<Url>) {
	*server_url_cell().lock().unwrap() = url.into();
}

/// Builds a [`Request`] to a server-action path using the configured
/// [`server_url`].
pub fn server_action_request(method: HttpMethod, path: &str) -> Request {
	let base = server_url().to_string();
	let url = format!("{}{}", base.trim_end_matches('/'), path);
	Request::new(method, url)
}

/// Sends a request whose handler returns a fallible result, splitting the
/// outcome by status code.
///
/// Mirrors [`JsonResult`]'s response encoding: a 2xx body decodes as `T`
/// (`Ok`), a body at `err_status` decodes as `E` (`Err`), and any other status
/// is a transport error. Use [`JsonResult::DEFAULT_ERR_STATUS`] for `err_status`
/// unless the handler overrides it.
#[cfg(feature = "json")]
pub async fn send_fallible<T, E>(
	request: Request,
	err_status: StatusCode,
) -> Result<Result<T, E>>
where
	T: serde::de::DeserializeOwned,
	E: serde::de::DeserializeOwned,
{
	parse_fallible_response(request.send().await?, err_status).await
}

/// Splits a [`JsonResult`]-encoded response into `Ok(T)`/`Err(E)` by status.
/// See [`send_fallible`].
#[cfg(feature = "json")]
pub async fn parse_fallible_response<T, E>(
	response: Response,
	err_status: StatusCode,
) -> Result<Result<T, E>>
where
	T: serde::de::DeserializeOwned,
	E: serde::de::DeserializeOwned,
{
	match response.status() {
		status if status == err_status => Ok(Err(response.json::<E>().await?)),
		status if status.is_ok() => Ok(Ok(response.json::<T>().await?)),
		_ => Err(response.into_error().await.into()),
	}
}

#[cfg(test)]
#[cfg(feature = "json")]
mod test {
	use super::*;

	fn ok_body<T: serde::Serialize>(value: &T) -> Response {
		Response::ok_body(
			serde_json::to_string(value).unwrap(),
			MediaType::Json,
		)
	}

	#[beet_core::test]
	async fn parses_ok() {
		parse_fallible_response::<i32, String>(
			ok_body(&8i32),
			JsonResult::DEFAULT_ERR_STATUS,
		)
		.await
		.unwrap()
		.unwrap()
		.xpect_eq(8);
	}

	#[beet_core::test]
	async fn parses_err_at_err_status() {
		let response = Response::from_status_body(
			JsonResult::DEFAULT_ERR_STATUS,
			serde_json::to_string("bad input").unwrap(),
			MediaType::Json,
		);
		parse_fallible_response::<i32, String>(
			response,
			JsonResult::DEFAULT_ERR_STATUS,
		)
		.await
		.unwrap()
		.unwrap_err()
		.xpect_eq("bad input");
	}

	#[beet_core::test]
	async fn other_status_is_transport_error() {
		let response = Response::from_status(StatusCode::INTERNAL_SERVER_ERROR);
		parse_fallible_response::<i32, String>(
			response,
			JsonResult::DEFAULT_ERR_STATUS,
		)
		.await
		.xpect_err();
	}
}
