use http::StatusCode;
use serde::Deserialize;
use serde::Serialize;
use sweet::net::exports::reqwest;

pub type ActionResult<T, E = String> = Result<T, ActionError<E>>;

/// An error that can be returned by a server action. Like an
/// `anyhow::Error`, this type does not implement `std::error::Error`,
/// which enables the `?` operator to be used in the server action handler.
#[derive(Debug, Serialize, Deserialize)]
pub struct ActionError<E = String> {
	pub status: u16,
	pub error: E,
}

impl<E> ActionError<E> {
	pub fn new(status: u16, error: E) -> Self { Self { status, error } }
}

impl<E: std::error::Error> From<E> for ActionError<String> {
	fn from(value: E) -> Self {
		Self {
			status: 400,
			error: value.to_string(),
		}
	}
}

impl<E: ToString> std::fmt::Display for ActionError<E> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"ActionError {{ status: {}, error: {} }}",
			self.status,
			self.error.to_string()
		)
	}
}

#[extend::ext(name=AnyhowToActionResult)]
pub impl<T, E: From<String>> anyhow::Result<T> {
	/// A helper method, shorthand for
	/// `.map_err(|err| ActionError::new(400, err.to_string().into()))`.
	fn into_action_result(self) -> ActionResult<T, E> {
		self.map_err(|err| err.into_action_error())
	}
}
#[extend::ext(name=AnyhowToActionError)]
pub impl<E: From<String>> anyhow::Error {
	/// A helper method, shorthand for ActionError::new(400,self.to_string().into())`.
	fn into_action_error(&self) -> ActionError<E> {
		ActionError::new(400, self.to_string().into())
	}
}


pub type ServerActionResult<T, E> = Result<T, ServerActionError<E>>;

/// Client side errors returend by [`CallServerAction`]. The
/// [ClientError](ServerActionError::ClientError) variant is used
/// for returning serialized errors from the server.
#[derive(Debug, thiserror::Error)]
pub enum ServerActionError<E = String> {
	/// The request body could not be serialized, ie `req.json()`
	#[error("Failed to serialize request:\nError: {0}")]
	Serialize(serde_json::Error),
	/// The request failed, often due to connection error
	#[error("Error making request:\nError: {0}")]
	Request(reqwest::Error),
	/// The response body could not be read, ie `res.bytes()`
	#[error("Error getting response body:\nError: {0}")]
	ResponseBody(reqwest::Error),
	/// The response was successful but the body could not be deserialized.
	#[error("Failed to deserialize response:\nError: {0}")]
	Deserialize(serde_json::Error),
	/// A 400 error from the server with a valid body of type `E`. If the
	/// error is not of type `E`, a 400 [`Deserialize`] error will be returned.
	#[error("Response returned an action error: {0}")]
	ActionError(ActionError<E>),
	/// This is a catch-all but should be a 4xx or 5xx error.
	#[error(
		"Response returned an unparsed error:\nStatus Code: {0}\nError: {1}"
	)]
	UnparsedError(StatusCode, String),
}

impl<E> Into<ServerActionError<E>> for ActionError<E> {
	fn into(self) -> ServerActionError<E> {
		ServerActionError::ActionError(self)
	}
}

// impl Into<ServerActionError> for reqwest::Error {
// 	fn into(self) -> ServerActionError { ServerActionError::Request(self) }
// }
