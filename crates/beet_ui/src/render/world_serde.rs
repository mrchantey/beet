use crate::prelude::*;
use beet_core::prelude::*;



/// A [`NodeRenderer`] that serializes the entity subtree via [`WorldSerdeSaver`].
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct WorldSerdeRenderer {}


impl NodeRenderer for WorldSerdeRenderer {
	fn render(
		&mut self,
		cx: &mut RenderContext,
	) -> Result<RenderOutput, RenderError> {
		for accepts in &cx.accepts {
			match accepts {
				#[cfg(feature = "json")]
				MediaType::Json => {
					let media_bytes = WorldSerdeSaver::new(cx.world)
						.with_entity_tree(cx.entity)
						.save(MediaType::Json)?;
					return RenderOutput::Media(media_bytes).xok();
				}
				#[cfg(feature = "postcard")]
				MediaType::Postcard => {
					let media_bytes = WorldSerdeSaver::new(cx.world)
						.with_entity_tree(cx.entity)
						.save(MediaType::Postcard)?;
					return RenderOutput::Media(media_bytes).xok();
				}
				_ => {}
			}
		}
		Err(RenderError::AcceptMismatch {
			requested: cx.accepts.clone(),
			available: vec![
				#[cfg(feature = "json")]
				MediaType::Json,
				#[cfg(feature = "postcard")]
				MediaType::Postcard,
			],
		})
	}
}
