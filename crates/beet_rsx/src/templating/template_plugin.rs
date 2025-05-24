use crate::prelude::*;
use bevy::ecs::schedule::SystemSet;
use bevy::prelude::*;
use std::cell::RefCell;

#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct SpawnStep;
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct ApplyTransformsStep;
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct RenderStep;

#[derive(Default)]
pub struct TemplatePlugin;


impl Plugin for TemplatePlugin {
	fn build(&self, app: &mut App) {
		app.configure_sets(
			Update,
			(
				ApplyTransformsStep.after(SpawnStep),
				RenderStep.after(ApplyTransformsStep),
				ApplySlotsStep.in_set(ApplyTransformsStep),
			),
		)
		.add_plugins((apply_slots_plugin, render_html_plugin));
	}
}



thread_local! {
	static TEMPLATE_APP: RefCell<Option<App>> = RefCell::new(None);
}


/// Access the thread local [`App`] used by the [`TemplatePlugin`].
pub struct TemplateApp;

impl TemplateApp {
	pub fn with<O>(func: impl FnOnce(&mut App) -> O) -> O {
		TEMPLATE_APP.with(|app_cell| {
			// Initialize the app if needed
			let mut app_ref = app_cell.borrow_mut();
			if app_ref.is_none() {
				let mut app = App::new();
				app.add_plugins(TemplatePlugin);
				*app_ref = Some(app);
			}

			// Now we can safely unwrap and use the app
			let app = app_ref.as_mut().unwrap();

			func(app)
		})
	}
}
