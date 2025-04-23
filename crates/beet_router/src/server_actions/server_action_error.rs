use crate::prelude::*;
use http::StatusCode;
use sweet::net::exports::reqwest;




/// Client side errors returend by [`CallServerAction`]. The 
/// [ClientError](ServerActionError::ClientError) variant is used 
/// for returning serialized errors from the server.
#[derive(Debug, thiserror::Error)]
pub enum ServerActionError<E = String> {
	/// The request body could not be serialized, ie `req.json()`
	#[error("Failed to serialize request:\nRoute: {0}\nError: {1}")]
	Serialize(RouteInfo, serde_json::Error),
	/// The request failed, often due to connection error
	#[error("Error making request:\nRoute: {0}\nError: {1}")]
	Request(RouteInfo, reqwest::Error),
	/// The response body could not be read, ie `res.bytes()`
	#[error("Error getting response body:\nRoute: {0}\nError: {1}")]
	ResponseBody(RouteInfo, reqwest::Error),
	#[error("Failed to deserialize response:\nRoute: {0}\nError: {1}")]
	Deserialize(RouteInfo, serde_json::Error),
	/// A 400-499 error from the server. The error type
	/// will be serialized as a JSON object.
	#[error(
		"Response returned a client error:\nRoute: {0}\nStatus Code: {1}\nError: {2}"
	)]
	ClientError(RouteInfo, StatusCode, E),
	/// A non-400 error from the server with no body.
	/// See server traces for more details.
	/// This is a catch-all but should be a 500-599 error.
	#[error("Response returned a server error:\nStatus Code: {1}\nRoute: {0}")]
	ServerError(RouteInfo, StatusCode),
}

// impl Into<ServerActionError> for reqwest::Error {
// 	fn into(self) -> ServerActionError { ServerActionError::Request(self) }
// }
