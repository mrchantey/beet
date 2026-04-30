use crate::prelude::*;
use beet_core::prelude::*;
use std::sync::Arc;


#[derive(Clone, Component)]
pub struct EntityWidget {
	render: Arc<dyn 'static + Send + Sync + Fn(TuiRenderContext) -> Result>,
}

impl EntityWidget {
	pub fn new(
		render: impl 'static + Send + Sync + Fn(TuiRenderContext) -> Result,
	) -> Self {
		Self {
			render: Arc::new(render),
		}
	}
	pub fn render(&mut self, cx: TuiRenderContext) -> Result {
		(self.render)(cx)
	}
}
