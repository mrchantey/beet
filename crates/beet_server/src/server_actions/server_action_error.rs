use beet_rsx::as_beet::HttpError;
use bevy::ecs::error::BevyError;

pub type ServerActionResult<T, E = String> = Result<T, ServerActionError<E>>;

/// Client side errors returend by [`CallServerAction`]. The
/// [ClientError](ServerActionError::ClientError) variant is used
/// for returning serialized errors from the server.
#[derive(Debug, thiserror::Error)]
pub enum ServerActionError<E = String> {
	#[error("{0}")]
	BevyError(BevyError),
	/// A 400 error from the server with a valid body of type `E`. If the
	/// error is not of type `E`, a 400 [`Deserialize`] error will be returned.
	#[error("{0}")]
	ActionError(E),
	#[error("{0}")]
	HttpError(HttpError),
}

impl<E> ServerActionError<E> {
	pub fn from_opaque(err: impl Into<BevyError>) -> Self {
		Self::BevyError(err.into())
	}
}


impl<E: 'static + Clone + std::error::Error> From<BevyError>
	for ServerActionError<E>
{
	fn from(value: BevyError) -> Self {
		if let Some(err) = value.downcast_ref::<E>() {
			Self::ActionError(err.clone())
		} else if let Some(err) = value.downcast_ref::<HttpError>() {
			ServerActionError::HttpError(err.clone())
		} else {
			ServerActionError::BevyError(value)
		}
	}
}
