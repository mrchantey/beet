use crate::prelude::*;
use beet_core::prelude::*;

/// A [`NodeRenderer`] that serializes the entity subtree via [`TemplateSaver`].
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct TemplateRenderer {}

impl TemplateRenderer {
	/// The serialization formats this build can produce, preferred first.
	pub fn available() -> Vec<MediaType> {
		vec![
			#[cfg(feature = "json")]
			MediaType::Json,
			#[cfg(feature = "postcard")]
			MediaType::Postcard,
		]
	}

	/// The format to serialize for `accepts`: the first accepted format this
	/// build produces, or the preferred one when `accepts` is empty, since an
	/// empty list accepts anything (see [`RenderContext::accepts`]).
	fn negotiate(accepts: &[MediaType]) -> Result<MediaType, RenderError> {
		let available = Self::available();
		match accepts.is_empty() {
			true => available.first().cloned(),
			false => accepts
				.iter()
				.find(|media_type| available.contains(media_type))
				.cloned(),
		}
		.ok_or_else(|| RenderError::AcceptMismatch {
			requested: accepts.to_vec(),
			available,
		})
	}
}

impl NodeRenderer for TemplateRenderer {
	fn render(
		&mut self,
		cx: &mut RenderContext,
	) -> Result<MediaBytes, RenderError> {
		let media_type = Self::negotiate(&cx.accepts)?;
		TemplateSaver::new()
			.with_entity_tree(cx.world, cx.entity)
			.save(cx.world, media_type)?
			.xok()
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// An empty accepts list accepts anything, so it serializes the preferred
	/// format rather than reporting a mismatch against nothing.
	#[cfg(feature = "json")]
	#[beet_core::test]
	fn empty_accepts_serializes_the_preferred_format() {
		let mut world = world_ext::ui_world();
		let entity = world.spawn_empty().id();
		let mut cx = RenderContext::new(entity, &mut world);
		TemplateRenderer::default()
			.render(&mut cx)
			.unwrap()
			.media_type()
			.xpect_eq(MediaType::Json);
	}

	#[beet_core::test]
	fn unsupported_accepts_mismatch() {
		let mut world = World::new();
		let entity = world.spawn_empty().id();
		let mut cx = RenderContext::new(entity, &mut world)
			.with_accepts(vec![MediaType::Png]);
		match TemplateRenderer::default().render(&mut cx) {
			Err(RenderError::AcceptMismatch { .. }) => {}
			other => panic!("expected AcceptMismatch, got {other:?}"),
		}
	}
}
