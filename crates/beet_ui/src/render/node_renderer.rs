use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use bevy::ecs::system::SystemState;
use thiserror::Error;

/// Renders an entity tree into serialized [`MediaBytes`].
///
/// Implementors walk the entity tree rooted at `cx.entity` using
/// `cx.walk()` and produce the serialized bytes for their media type.
pub trait NodeRenderer {
	/// Render the entity tree described by `cx`.
	fn render(
		&mut self,
		cx: &mut RenderContext,
	) -> Result<MediaBytes, RenderError>;
}

/// Context passed to [`NodeRenderer::render`]: the tree to render and the
/// request the render answers.
///
/// Every render answers a request, a test's being
/// [`RequestParts::default()`], which accepts anything.
pub struct RenderContext<'a> {
	/// The world containing the entity tree.
	pub world: &'a mut World,
	/// The entity to render.
	pub entity: Entity,
	/// The request this render answers.
	pub request: &'a RequestParts,
	/// The media type a [`RenderTargets`] negotiation chose, which the
	/// request's `Accept` need not name literally (a wildcard's default, the
	/// plain text fallback).
	negotiated: Option<MediaType>,
}

impl<'a> RenderContext<'a> {
	/// Create a new [`RenderContext`] rendering `entity` as the answer to
	/// `request`.
	pub fn new(
		world: &'a mut World,
		entity: Entity,
		request: &'a RequestParts,
	) -> Self {
		Self {
			world,
			entity,
			request,
			negotiated: None,
		}
	}

	/// Set the media type a negotiation chose for this render.
	pub(crate) fn with_negotiated(mut self, media_type: MediaType) -> Self {
		self.negotiated = Some(media_type);
		self
	}

	pub fn entity(&mut self) -> EntityRef<'_> { self.world.entity(self.entity) }

	pub fn entity_mut(&mut self) -> EntityWorldMut<'_> {
		self.world.entity_mut(self.entity)
	}

	/// The media types this render may answer, highest priority first: the
	/// negotiated one when a [`RenderTargets`] render chose it, else the
	/// request's `Accept`. Empty accepts anything.
	pub fn accepts(&self) -> Vec<MediaType> {
		match &self.negotiated {
			Some(media_type) => vec![media_type.clone()],
			None => self.request.accept(),
		}
	}

	/// Walk the entity tree rooted at [`Self::entity`], visiting each
	/// node with the provided [`NodeVisitor`].
	pub fn walk(&mut self, visitor: &mut impl NodeVisitor) {
		let mut state = SystemState::<NodeWalker>::new(self.world);
		let walker = state.get(self.world).expect("infallible node query");
		walker.walk(visitor, self.entity);
	}

	/// Check whether this render may answer as one of `available`.
	///
	/// Returns `Ok(())` if [`accepts`](Self::accepts) is empty (meaning any
	/// type is fine), contains a wildcard, or names at least one of
	/// `available`. Otherwise returns [`RenderError::AcceptMismatch`].
	pub fn check_accepts(
		&self,
		available: &[MediaType],
	) -> Result<(), RenderError> {
		let accepts = self.accepts();
		match accepts.is_empty()
			|| accepts.iter().any(|media_type| {
				media_type.is_wildcard() || available.contains(media_type)
			}) {
			true => Ok(()),
			false => Err(RenderError::AcceptMismatch {
				requested: accepts,
				available: available.to_vec(),
			}),
		}
	}
}

/// Error returned by [`NodeRenderer::render`].
#[derive(Debug, Error)]
pub enum RenderError {
	/// The renderer does not support any of the requested media type.
	#[error(
		"accept mismatch: requested {requested:?}, available {available:?}"
	)]
	AcceptMismatch {
		/// The media type requested by the caller.
		requested: Vec<MediaType>,
		/// The media type this renderer can produce.
		available: Vec<MediaType>,
	},
	/// Any other render failure.
	#[error("{0}")]
	Other(BevyError),
}

impl From<BevyError> for RenderError {
	fn from(err: BevyError) -> Self { RenderError::Other(err) }
}

/// A mismatch is the request's to fix, a `406 Not Acceptable`; any other
/// failure is the server's.
impl From<RenderError> for HttpError {
	fn from(err: RenderError) -> Self {
		match err {
			RenderError::AcceptMismatch { .. } => {
				HttpError::new(StatusCode::NOT_ACCEPTABLE, err.to_string())
			}
			RenderError::Other(err) => HttpError::from_opaque(err),
		}
	}
}
