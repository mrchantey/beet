use crate::prelude::*;
use beet_core::prelude::*;
use bevy::ecs::system::SystemState;
use thiserror::Error;

/// Renders an entity tree into a [`RenderOutput`].
///
/// Implementors walk the entity tree rooted at `cx.entity` using
/// `cx.walk()` and produce either serialized [`MediaBytes`] or a
/// [`RenderOutput::Stateful`] signal for persistent renderers.
pub trait NodeRenderer {
	/// Render the entity tree described by `cx`.
	fn render(
		&mut self,
		cx: &mut RenderContext,
	) -> Result<RenderOutput, RenderError>;


	fn run(
		&mut self,
		entity: &mut EntityWorldMut,
		accepts: Vec<MediaType>,
	) -> Result<RenderOutput, RenderError> {
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
		let walker = state.get(self.world);
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

/// The result of a [`NodeRenderer::render`] call.
#[derive(Debug)]
pub enum RenderOutput {
	/// The render produced typed bytes, ie html, markdown, json.
	Media(MediaBytes),
	/// The renderer is stateful (ie a persistent UI) and completed
	/// its render pass, which may or may not have changed the display.
	Stateful,
}

impl RenderOutput {
	/// Convenience constructor for a [`RenderOutput::Media`] with the given
	/// media type and UTF-8 string content.
	pub fn media_string(media_type: MediaType, content: String) -> Self {
		Self::Media(MediaBytes::new_string(media_type, content))
	}

	/// Returns the inner [`MediaBytes`] if this is a [`RenderOutput::Media`].
	pub fn media_bytes(&self) -> Option<&MediaBytes> {
		match self {
			Self::Media(mb) => Some(mb),
			Self::Stateful => None,
		}
	}

	/// Returns `true` if this is a [`RenderOutput::Media`] variant.
	pub fn is_media(&self) -> bool { matches!(self, Self::Media(_)) }

	/// Returns `true` if this is a [`RenderOutput::Stateful`] variant.
	pub fn is_stateful(&self) -> bool { matches!(self, Self::Stateful) }
}

impl core::fmt::Display for RenderOutput {
	fn fmt(
		&self,
		formatter: &mut core::fmt::Formatter<'_>,
	) -> core::fmt::Result {
		match self {
			Self::Media(mb) => write!(formatter, "{mb}"),
			Self::Stateful => write!(formatter, "Stateful"),
		}
	}
}


#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn display_media_utf8() {
		RenderOutput::media_string(MediaType::Html, "hello".into())
			.to_string()
			.xpect_eq("hello".to_string());
	}

	#[test]
	fn display_media_binary() {
		RenderOutput::Media(MediaBytes::new(MediaType::Bytes, vec![
			0xFF, 0xFE,
		]))
		.to_string()
		.xpect_eq("<2 bytes of application/octet-stream>".to_string());
	}

	#[test]
	fn display_stateful() {
		RenderOutput::Stateful
			.to_string()
			.xpect_eq("Stateful".to_string());
	}

	#[test]
	fn as_str_media() {
		RenderOutput::media_string(MediaType::Text, "hello".into())
			.to_string()
			.xpect_eq("hello");
	}

	#[test]
	fn media_bytes_accessor() {
		let output =
			RenderOutput::media_string(MediaType::Json, r#"{"a":1}"#.into());
		output.media_bytes().is_some().xpect_true();
		RenderOutput::Stateful.media_bytes().is_none().xpect_true();
	}
}
