use crate::prelude::*;
use once_cell::sync::Lazy;
use reqwest::Client;
use reqwest::Method;
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

	/// Makes a HTTP request to a server action.
	/// Automatically uses the correct request style based on the HTTP method:
	/// - Bodyless methods (GET, HEAD, DELETE, OPTIONS, CONNECT, TRACE) send data as query parameters
	/// - Methods with body (POST, PUT, PATCH) send data in the request body
	pub async fn request<T: Serialize, O: DeserializeOwned>(
		method: &str,
		path: impl Into<RoutePath>,
		value: T,
	) -> Result<O, CallServerActionError> {
		// Convert method string to reqwest Method
		let method = match method.to_uppercase().as_str() {
			"GET" => Method::GET,
			"POST" => Method::POST,
			"PUT" => Method::PUT,
			"DELETE" => Method::DELETE,
			"HEAD" => Method::HEAD,
			"OPTIONS" => Method::OPTIONS,
			"CONNECT" => Method::CONNECT,
			"PATCH" => Method::PATCH,
			"TRACE" => Method::TRACE,
			_ => {
				return Err(CallServerActionError::InvalidMethod(
					method.to_string(),
				));
			}
		};

		// Determine if this is a bodyless method
		let is_bodyless = matches!(
			method,
			Method::GET
				| Method::HEAD
				| Method::DELETE
				| Method::OPTIONS
				| Method::CONNECT
				| Method::TRACE
		);

		if is_bodyless {
			Self::request_with_query(method, path, value).await
		} else {
			Self::request_with_body(method, path, value).await
		}
	}

	/// Internal function to make a request with data in the query parameters.
	/// Used by GET, HEAD, DELETE, OPTIONS, CONNECT, TRACE methods.
	async fn request_with_query<T: Serialize, O: DeserializeOwned>(
		method: Method,
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
		method: Method,
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
	#[error("Invalid HTTP method: {0}")]
	InvalidMethod(String),
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
