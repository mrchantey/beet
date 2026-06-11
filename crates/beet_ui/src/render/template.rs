use crate::prelude::*;
use beet_core::prelude::*;



/// A [`NodeRenderer`] that serializes the entity subtree via [`TemplateSaver`].
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct TemplateRenderer {}


impl NodeRenderer for TemplateRenderer {
	fn render(
		&mut self,
		cx: &mut RenderContext,
	) -> Result<MediaBytes, RenderError> {
		for accepts in &cx.accepts {
			match accepts {
				#[cfg(feature = "json")]
				MediaType::Json => {
					return TemplateSaver::new()
						.with_entity_tree(cx.world, cx.entity)
						.save(cx.world, MediaType::Json)?
						.xok();
				}
				#[cfg(feature = "postcard")]
				MediaType::Postcard => {
					return TemplateSaver::new()
						.with_entity_tree(cx.world, cx.entity)
						.save(cx.world, MediaType::Postcard)?
						.xok();
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
