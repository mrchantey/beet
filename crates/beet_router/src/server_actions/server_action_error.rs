use beet_core::net::cross_fetch;
use http::StatusCode;
use serde::Deserialize;
use serde::Serialize;

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

pub trait IntoActionResult<T, E, M> {
	fn into_action_result(self) -> ActionResult<T, E>;
}
impl<T, E> IntoActionResult<T, E, Self> for Result<T, ActionError<E>> {
	fn into_action_result(self) -> ActionResult<T, E> { self }
}

impl<T> IntoActionResult<T, String, Self> for Result<T, anyhow::Error> {
	fn into_action_result(self) -> ActionResult<T, String> {
		self.map_err(|err| ActionError::new(400, err.to_string()))
	}
}

impl<T, E: ToString> IntoActionResult<T, String, Self> for (StatusCode, E) {
	fn into_action_result(self) -> ActionResult<T, String> {
		let (status, error) = self;
		Err(ActionError::new(status.as_u16(), error.to_string()))
	}
}


#[extend::ext(name=AnyhowToActionError)]
pub impl<E: From<String>> anyhow::Error {
	/// A helper method, shorthand for ActionError::new(400,self.to_string().into())`.
	fn into_action_error(&self) -> ActionError<E> {
		ActionError::new(400, self.to_string().into())
	}
}


pub type ServerActionResult<T, E = String> = Result<T, ServerActionError<E>>;

/// Client side errors returend by [`CallServerAction`]. The
/// [ClientError](ServerActionError::ClientError) variant is used
/// for returning serialized errors from the server.
#[derive(Debug, thiserror::Error)]
pub enum ServerActionError<E = String> {
	#[error("{0}")]
	FetchError(#[from] cross_fetch::Error),
	/// A 400 error from the server with a valid body of type `E`. If the
	/// error is not of type `E`, a 400 [`Deserialize`] error will be returned.
	#[error("Response returned an action error: {0}")]
	ActionError(E),
	/// This is a catch-all but should be a 4xx or 5xx error.
	#[error(
		"Response returned an unparsed error:\nStatus Code: {0}\nError: {1}"
	)]
	UnparsedError(StatusCode, String),
}

impl<E> Into<ServerActionError<E>> for ActionError<E> {
	fn into(self) -> ServerActionError<E> {
		ServerActionError::ActionError(self.error)
	}
}

// impl Into<ServerActionError> for reqwest::Error {
// 	fn into(self) -> ServerActionError { ServerActionError::Request(self) }
// }
