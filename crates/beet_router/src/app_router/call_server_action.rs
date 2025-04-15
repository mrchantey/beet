use crate::prelude::*;
use once_cell::sync::Lazy;
use reqwest::Client;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::sync::Mutex;


static SERVER_URL: Lazy<Mutex<RoutePath>> =
	Lazy::new(|| Mutex::new("/".into()));

pub struct CallServerAction;

static CLIENT: Lazy<Client> = Lazy::new(|| Client::new());

impl CallServerAction {
	pub fn get_server_url() -> RoutePath { SERVER_URL.lock().unwrap().clone() }
	pub fn set_server_url(url: RoutePath) { *SERVER_URL.lock().unwrap() = url; }

	/// Internal function to make a request with data in the query parameters.
	/// Used by GET, HEAD, DELETE, OPTIONS, CONNECT, TRACE methods.
	async fn request_with_query<T: Serialize, O: DeserializeOwned>(
		method: reqwest::Method,
		path: impl Into<RoutePath>,
		value: T,
	) -> Result<O, CallServerActionError> {
		let value = serde_json::to_string(&value)
			.map_err(|e| CallServerActionError::Serialize(e))?;

		let url = SERVER_URL.lock().unwrap().join(&path.into());
		let bytes = CLIENT
			.request(method, url.to_string())
			.query(&[("data", value)])
			.send()
			.await
			.map_err(|e| e.into())?
			.error_for_status()
			.map_err(|e| CallServerActionError::Response(e.to_string()))?
			.bytes()
			.await
			.map_err(|e| e.into())?;

		serde_json::from_slice(&bytes)
			.map_err(|e| CallServerActionError::Deserialize(e))
	}

	/// Internal function to make a request with data in the request body.
	/// Used by POST, PUT, PATCH methods.
	async fn request_with_body<T: Serialize, O: DeserializeOwned>(
		method: reqwest::Method,
		path: impl Into<RoutePath>,
		value: T,
	) -> Result<O, CallServerActionError> {
		let value = serde_json::to_string(&value)
			.map_err(|e| CallServerActionError::Serialize(e))?;

		let url = SERVER_URL.lock().unwrap().join(&path.into());
		let bytes = CLIENT
				.request(method, url.to_string())
				.header("Content-Type", "application/json")
				.body(value)
				.send()
				.await
				.map_err(|e| e.into())?
				.error_for_status()
				.map_err(|e| CallServerActionError::Response(e.to_string()))?
				.bytes()
				.await
				.map_err(|e| e.into())?;

		serde_json::from_slice(&bytes)
			.map_err(|e| CallServerActionError::Deserialize(e))
	}

	/// Call a server action with a GET request.
	/// The `value` is serialized to JSON and sent as a query parameter called `data`.
	pub async fn get<T: Serialize, O: DeserializeOwned>(
		path: impl Into<RoutePath>,
		value: T,
	) -> Result<O, CallServerActionError> {
		Self::request_with_query(reqwest::Method::GET, path, value).await
	}

	/// Call a server action with a HEAD request.
	/// Similar to GET but without returning a response body.
	/// The `value` is serialized to JSON and sent as a query parameter called `data`.
	pub async fn head<T: Serialize, O: DeserializeOwned>(
		path: impl Into<RoutePath>,
		value: T,
	) -> Result<O, CallServerActionError> {
		Self::request_with_query(reqwest::Method::HEAD, path, value).await
	}

	/// Call a server action with a POST request.
	/// The `value` is serialized to JSON and sent in the request body.
	pub async fn post<T: Serialize, O: DeserializeOwned>(
		path: impl Into<RoutePath>,
		value: T,
	) -> Result<O, CallServerActionError> {
		Self::request_with_body(reqwest::Method::POST, path, value).await
	}

	/// Call a server action with a PUT request.
	/// The `value` is serialized to JSON and sent in the request body.
	pub async fn put<T: Serialize, O: DeserializeOwned>(
		path: impl Into<RoutePath>,
		value: T,
	) -> Result<O, CallServerActionError> {
		Self::request_with_body(reqwest::Method::PUT, path, value).await
	}

	/// Call a server action with a DELETE request.
	/// The `value` is serialized to JSON and sent as a query parameter
	/// since DELETE requests typically don't have a body.
	pub async fn delete<T: Serialize, O: DeserializeOwned>(
		path: impl Into<RoutePath>,
		value: T,
	) -> Result<O, CallServerActionError> {
		Self::request_with_query(reqwest::Method::DELETE, path, value).await
	}

	/// Call a server action with a PATCH request.
	/// The `value` is serialized to JSON and sent in the request body.
	pub async fn patch<T: Serialize, O: DeserializeOwned>(
		path: impl Into<RoutePath>,
		value: T,
	) -> Result<O, CallServerActionError> {
		Self::request_with_body(reqwest::Method::PATCH, path, value).await
	}

	/// Call a server action with an OPTIONS request.
	/// The `value` is serialized to JSON and sent as a query parameter called `data`.
	pub async fn options<T: Serialize, O: DeserializeOwned>(
		path: impl Into<RoutePath>,
		value: T,
	) -> Result<O, CallServerActionError> {
		Self::request_with_query(reqwest::Method::OPTIONS, path, value).await
	}

	/// Call a server action with a CONNECT request.
	/// The `value` is serialized to JSON and sent as a query parameter called `data`.
	pub async fn connect<T: Serialize, O: DeserializeOwned>(
		path: impl Into<RoutePath>,
		value: T,
	) -> Result<O, CallServerActionError> {
		Self::request_with_query(reqwest::Method::CONNECT, path, value).await
	}

	/// Call a server action with a TRACE request.
	/// The `value` is serialized to JSON and sent as a query parameter called `data`.
	pub async fn trace<T: Serialize, O: DeserializeOwned>(
		path: impl Into<RoutePath>,
		value: T,
	) -> Result<O, CallServerActionError> {
		Self::request_with_query(reqwest::Method::TRACE, path, value).await
	}
}

#[derive(Debug, thiserror::Error)]
pub enum CallServerActionError {
	#[error("Error making request: {0}")]
	Request(reqwest::Error),
	#[error("Response returned a non-200 error: {0}")]
	Response(String),
	#[error("Failed to serialize request: {0}")]
	Serialize(serde_json::Error),
	#[error("Failed to deserialize response: {0}")]
	Deserialize(serde_json::Error),
}

impl Into<CallServerActionError> for reqwest::Error {
	fn into(self) -> CallServerActionError {
		CallServerActionError::Request(self)
	}
}


/// tests in crates/beet_server/src/tests/call_server_action.rs
/// they depend on beet_server::JsonQuery
#[cfg(test)]
mod tests {}
