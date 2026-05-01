use crate::prelude::*;
use beet_core::prelude::*;


#[derive(Component, Deref, DerefMut)]
pub struct EntityWidget {
	widget: Box<dyn 'static + Send + Sync + Widget>,
}

impl EntityWidget {
	pub fn new(render: impl 'static + Send + Sync + Widget) -> Self {
		Self {
			widget: Box::new(render),
		}
	}
}
