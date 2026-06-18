use crate::prelude::*;
use beet_core::prelude::*;
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

	fn run(
		&mut self,
		entity: &mut EntityWorldMut,
		accepts: Vec<MediaType>,
	) -> Result<MediaBytes, RenderError> {
		let id = entity.id();
		entity.world_scope(|world| {
			self.render(
				&mut RenderContext::new(id, world).with_accepts(accepts),
			)
		})
	}
}

/// Context passed to [`NodeRenderer::render`].
pub struct RenderContext<'a> {
	/// The entity to render.
	pub entity: Entity,
	/// The world containing the entity tree.
	pub world: &'a mut World,
	/// Ordered list of acceptable output types, highest priority first.
	/// An empty vec means any type is acceptable.
	pub accepts: Vec<MediaType>,
}

impl<'a> RenderContext<'a> {
	/// Create a new [`RenderContext`] with the given entity and world.
	pub fn new(entity: Entity, world: &'a mut World) -> Self {
		Self {
			entity,
			world,
			accepts: Vec::new(),
		}
	}

	pub fn entity(&mut self) -> EntityRef<'_> { self.world.entity(self.entity) }

	pub fn entity_mut(&mut self) -> EntityWorldMut<'_> {
		self.world.entity_mut(self.entity)
	}

	/// Set the accepted media types.
	pub fn with_accepts(mut self, accepts: Vec<MediaType>) -> Self {
		self.accepts = accepts;
		self
	}

	/// Walk the entity tree rooted at [`Self::entity`], visiting each
	/// node with the provided [`NodeVisitor`].
	pub fn walk(&mut self, visitor: &mut impl NodeVisitor) {
		let mut state = SystemState::<NodeWalker>::new(self.world);
		let walker = state.get(self.world).expect("infallible node query");
		walker.walk(visitor, self.entity);
	}

	/// Check whether the `accepts` list is compatible with the given
	/// `available` media type.
	///
	/// Returns `Ok(())` if `accepts` is empty (meaning any type is fine),
	/// if `accepts` contains the HTTP `*/*` wildcard, or if at least one
	/// entry in `accepts` matches one of `available`.
	/// Otherwise returns [`RenderError::AcceptMismatch`].
	pub fn check_accepts(
		&self,
		available: &[MediaType],
	) -> Result<(), RenderError> {
		if self.accepts.is_empty() {
			return Ok(());
		}
		if self
			.accepts
			.iter()
			.any(|mt| mt.is_wildcard() || available.contains(mt))
		{
			return Ok(());
		}
		Err(RenderError::AcceptMismatch {
			requested: self.accepts.clone(),
			available: available.to_vec(),
		})
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
