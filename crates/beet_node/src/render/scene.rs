use crate::prelude::*;
use beet_core::prelude::*;



#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SceneRenderer {}


impl NodeRenderer for SceneRenderer {
	fn render(
		&mut self,
		cx: &mut RenderContext,
	) -> Result<RenderOutput, RenderError> {
		for accepts in &cx.accepts {
			match accepts {
				#[cfg(feature = "json")]
				MediaType::Json => {
					let bytes = SceneSaver::new(cx.world)
						.with_entity_tree(cx.entity)
						.save_json()?;
					return RenderOutput::Media(MediaBytes::new(
						MediaType::Json,
						bytes,
					))
					.xok();
				}
				#[cfg(feature = "postcard")]
				MediaType::Postcard => {
					let bytes = SceneSaver::new(cx.world)
						.with_entity_tree(cx.entity)
						.save_postcard()?;
					return RenderOutput::Media(MediaBytes::new(
						MediaType::Postcard,
						bytes,
					))
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
