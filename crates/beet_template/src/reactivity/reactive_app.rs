use bevy::app::PluginsState;
use bevy::prelude::*;
use std::cell::RefCell;

/// By default an app will run in a continuous loop,
/// but for ui applications we usually only want to run it after
/// some user input.
pub struct ReactiveApp;
thread_local! {
	static APP: RefCell<Option<App>> = RefCell::new(None);
}

impl ReactiveApp {
	/// Consume the app, running it once then
	/// storing it in a [`thread_local`] and returning immediately.
	pub fn runner(mut app: App) -> AppExit {
		while app.plugins_state() == PluginsState::Adding {
			#[cfg(not(target_arch = "wasm32"))]
			bevy::tasks::tick_global_task_pools_on_main_thread();
		}
		app.finish();
		app.cleanup();
		app.update();
		APP.with(move |app_ref| {
			let mut app_cell = app_ref.borrow_mut();
			*app_cell = Some(app);
		});
		AppExit::Success
	}
	/// Access the thread local [`App`]
	/// # Panics
	/// Panics if the app is not initialized.
	pub fn with<O>(func: impl FnOnce(&mut App) -> O) -> O {
		APP.with(|app_ref| {
			let mut app_cell = app_ref.borrow_mut();
			let app = app_cell.as_mut().expect(
				"ReactiveApp not initialized. plese call app.set_runner(ReactiveApp::runner).run()");
				func(app)
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
}
