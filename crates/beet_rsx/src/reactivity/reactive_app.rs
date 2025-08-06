use beet_core::prelude::*;
use bevy::prelude::*;
use std::cell::RefCell;
use std::sync::Arc;
use std::sync::LazyLock;
use std::sync::RwLock;

use crate::prelude::ApplyDirectivesPlugin;

/// Temporary solution until reactivity in bevy.
pub struct ReactiveApp;
thread_local! {
	static APP: RefCell<Option<App>> = RefCell::new(None);
}

/// Machinery allowing downstream crates to add plugins to the app
static CREATE_APP: LazyLock<
	Arc<RwLock<Box<dyn 'static + Send + Sync + Fn() -> App>>>,
> = LazyLock::new(|| {
	Arc::new(RwLock::new(Box::new(|| {
		let mut app = App::new();
		app.add_plugins(ApplyDirectivesPlugin);
		app
	})))
});

impl ReactiveApp {
	pub fn set_create_app(func: impl 'static + Send + Sync + Fn() -> App) {
		*CREATE_APP.write().unwrap() = Box::new(func);
	}

	/// Consume the app, running it once then
	/// storing it in a [`thread_local`] and returning immediately.
	pub fn runner(mut app: App) -> AppExit {
		PrettyTracing::default().init();
		app.init();
		app.update();
		APP.with(move |app_ref| {
			let mut app_cell = app_ref.borrow_mut();
			*app_cell = Some(app);
		});
		AppExit::Success
	}
	/// Access the thread local [`App`], initializing it if it is not already.
	pub fn with<O>(func: impl FnOnce(&mut App) -> O) -> O {
		APP.with(|app_ref| {
			let mut app_cell = app_ref.borrow_mut();
			match app_cell.as_mut() {
				Some(app) => func(app),
				None => {
					*app_cell = Some(CREATE_APP.read().unwrap()());
					let mut app = app_cell.as_mut().unwrap();
					let out = func(&mut app);
					out
				}
			}
		})
	}

	/// Try to access the thread local [`App`], returns None if
	/// already borrowed or uninitialized.
	pub fn try_with<O>(func: impl FnOnce(&mut App) -> O) -> Option<O> {
		APP.with(|app_ref| {
			if let Ok(mut app_cell) = app_ref.try_borrow_mut() {
				let app = app_cell.as_mut()?;
				Some(func(app))
			} else {
				None
			}
		})
	}

	/// Run [`App::update`] once and return [`App::should_exit`].
	/// # Panics
	/// Panics if the app is not initialized.
	pub fn update() -> Option<AppExit> {
		Self::with(|app| {
			app.update();
			app.should_exit()
		})
	}

	/// Try to run [`App::update`] once, returns None if app is not initialized.
	pub fn try_update() -> Option<Option<AppExit>> {
		Self::try_with(|app| {
			app.update();
			app.should_exit()
		})
	}
	/// Wrapper for [`World::insert_resource`].
	pub fn insert_resource<T: Resource>(resource: T) {
		Self::with(|app| app.world_mut().insert_resource(resource));
	}

	/// Wrapper for [`World::resource`].
	pub fn resource<T: Clone + Resource>() -> T {
		Self::with(|app| app.world().resource::<T>().clone())
	}
}
