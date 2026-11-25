use crate::prelude::*;
use beet_core::prelude::*;
#[cfg(feature = "scene")]
use beet_scene::prelude::*;

/// A [`DocRenderer`] that serializes the entity subtree via [`TemplateSaver`],
/// as the scene whose root the entity is: a parent above it is no part of the
/// document.
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

impl DocRenderer for TemplateRenderer {
	fn media_types(&self) -> Vec<MediaType> { Self::available() }

	fn render(
		&mut self,
		cx: &mut RenderContext,
	) -> Result<MediaBytes, RenderError> {
		let media_type = Self::negotiate(&cx.accepts())?;
		TemplateSaver::new()
			.save_roots_mapped(cx.world, media_type, &[cx.entity], default())?
			.0
			.xok()
	}
}

#[cfg(feature = "scene")]
impl Installer for TemplateRenderer {
	fn install(self, app: &mut App) { DocRenderers::register_on(app, self); }
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	/// A request with no `Accept` accepts anything, so it serializes the
	/// preferred format rather than reporting a mismatch against nothing.
	#[cfg(feature = "json")]
	#[beet_core::test]
	fn no_accept_serializes_the_preferred_format() {
		let mut world = world_ext::ui_world();
		let entity = world.spawn_empty().id();
		let request = RequestParts::default();
		let mut cx = RenderContext::new(&mut world, entity, &request);
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
		let request = RequestParts::default().with_accept(MediaType::Png);
		let mut cx = RenderContext::new(&mut world, entity, &request);
		match TemplateRenderer::default().render(&mut cx) {
			Err(RenderError::AcceptMismatch { .. }) => {}
			other => panic!("expected AcceptMismatch, got {other:?}"),
		}
	}
}
