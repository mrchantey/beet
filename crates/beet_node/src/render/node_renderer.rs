use crate::prelude::*;
use beet_core::prelude::*;
use thiserror::Error;

/// Renders an entity tree into a [`RenderOutput`].
///
/// Implementors walk the entity tree rooted at `cx.entity` using
/// `cx.walker` and produce either serialized [`MediaBytes`] or a
/// [`RenderOutput::Stateful`] signal for persistent renderers.
pub trait NodeRenderer {
	/// Render the entity tree described by `cx`.
	fn render(
		&mut self,
		cx: &RenderContext,
	) -> Result<RenderOutput, RenderError>;


	fn run(
		&self,
		entity: &mut EntityWorldMut,
		accepts: Vec<MediaType>,
	) -> Result<RenderOutput, RenderError>
	where
		Self: 'static + Sized + Clone,
	{
		let id = entity.id();
		entity.world_scope(|world| {
			world
				.run_system_cached_with(
					move |In((mut renderer, entity, accepts)): In<(
						Self,
						Entity,
						Vec<MediaType>,
					)>,
					      walker: NodeWalker| {
						// 3. Render to the requested media type
						renderer.render(
							&RenderContext::new(entity, &walker)
								.with_accepts(accepts),
						)
					},
					(self.clone(), id, accepts),
				)
				// no fallible systemparams
				.unwrap()
		})
	}
}



/// Context passed to [`NodeRenderer::render`].
pub struct RenderContext<'a> {
	/// The entity to render.
	pub entity: Entity,
	/// Walker for traversing the entity tree.
	pub walker: &'a NodeWalker<'a, 'a>,
	/// Ordered list of acceptable output types, highest priority first.
	/// An empty vec means any type is acceptable.
	pub accepts: Vec<MediaType>,
}

impl<'a> RenderContext<'a> {
	/// Create a new [`RenderContext`] with the given entity and walker.
	pub fn new(entity: Entity, walker: &'a NodeWalker<'a, 'a>) -> Self {
		Self {
			entity,
			walker,
			accepts: Vec::new(),
		}
	}

	/// Set the accepted media types.
	pub fn with_accepts(mut self, accepts: Vec<MediaType>) -> Self {
		self.accepts = accepts;
		self
	}

	/// Check whether the `accepts` list is compatible with the given
	/// `available` media type.
	///
	/// Returns `Ok(())` if `accepts` is empty (meaning any type is fine)
	/// or if at least one entry in `accepts` matches one of `available`.
	/// Otherwise returns [`RenderError::AcceptMismatch`].
	pub fn check_accepts(
		&self,
		available: &[MediaType],
	) -> Result<(), RenderError> {
		if self.accepts.is_empty() {
			return Ok(());
		}
		if self.accepts.iter().any(|mt| available.contains(mt)) {
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
	Media(MediaBytes<'static>),
	/// The renderer is stateful (ie a persistent UI) and completed
	/// its render pass, which may or may not have changed the display.
	Stateful,
}

impl RenderOutput {
	/// Convenience constructor for a [`RenderOutput::Media`] with the given
	/// media type and UTF-8 string content.
	pub fn media_string(media_type: MediaType, content: String) -> Self {
		Self::Media(MediaBytes::from_string(media_type, content))
	}

	/// Returns the inner [`MediaBytes`] if this is a [`RenderOutput::Media`].
	pub fn media_bytes(&self) -> Option<&MediaBytes<'static>> {
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
