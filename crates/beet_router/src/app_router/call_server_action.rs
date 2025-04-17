use crate::prelude::*;
use once_cell::sync::Lazy;
use reqwest::Client;
use reqwest::RequestBuilder;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::sync::Mutex;


static SERVER_URL: Lazy<Mutex<RoutePath>> =
	Lazy::new(|| Mutex::new("http://localhost:3000".into()));

pub struct CallServerAction;

static CLIENT: Lazy<Client> = Lazy::new(|| Client::new());

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
		Self::send(CLIENT.request(route_info.method.into(), url.to_string()))
			.await
	}

	/// Internal function to make a request with data in the query parameters.
	/// Used by GET, HEAD, DELETE, OPTIONS, CONNECT, TRACE methods.
	async fn request_with_query<T: Serialize, O: DeserializeOwned>(
		route_info: RouteInfo,
		value: T,
	) -> Result<O, ServerActionError> {
		let value = serde_json::to_string(&value)
			.map_err(|e| ServerActionError::Serialize(e))?;

		let url = SERVER_URL.lock().unwrap().join(&route_info.path);
		Self::send(
			CLIENT
				.request(route_info.method.into(), url.to_string())
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
			.map_err(|e| ServerActionError::Serialize(e))?;

		let url = SERVER_URL.lock().unwrap().join(&route_info.path);
		Self::send(
			CLIENT
				.request(route_info.method.into(), url.to_string())
				.header("Content-Type", "application/json")
				.body(value),
		)
		.await
	}


	async fn send<O: DeserializeOwned>(
		request: RequestBuilder,
	) -> Result<O, ServerActionError> {
		let bytes = request
			.send()
			.await
			.map_err(|e| e.into())?
			.error_for_status()
			.map_err(|e| ServerActionError::Response(e.to_string()))?
			.bytes()
			.await
			.map_err(|e| e.into())?;

		serde_json::from_slice(&bytes)
			.map_err(|e| ServerActionError::Deserialize(e))
	}
}

#[derive(Debug, thiserror::Error)]
pub enum ServerActionError {
	#[error("Error making request: {0}")]
	Request(reqwest::Error),
	#[error("Response returned a non-200 error: {0}")]
	Response(String),
	#[error("Failed to serialize request: {0}")]
	Serialize(serde_json::Error),
	#[error("Failed to deserialize response: {0}")]
	Deserialize(serde_json::Error),
	#[error("Invalid HTTP method: {0}")]
	InvalidMethod(String),
}

impl Into<ServerActionError> for reqwest::Error {
	fn into(self) -> ServerActionError { ServerActionError::Request(self) }
}


/// tests in crates/beet_server/src/tests/call_server_action.rs
/// they depend on beet_server::JsonQuery
#[cfg(test)]
mod tests {}
