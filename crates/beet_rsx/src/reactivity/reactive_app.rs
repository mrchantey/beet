use crate::prelude::*;
use beet_core::prelude::*;
use bevy::prelude::*;
use std::cell::RefCell;
use std::sync::Arc;
use std::sync::LazyLock;
use std::sync::RwLock;

/// A thread local application designed for run-on-event patterns
/// like the DOM. The default app simply adds the [`ApplyDirectivesPlugin`]
/// but this can be overridden with [`ReactiveApp::set_create_app`]
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
		app.add_plugins((ApplyDirectivesPlugin, SignalsPlugin));
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

	/// Currently the native version of [`Self::queue_update`] is simply
	/// [`Self::try_update`]
	#[cfg(not(target_arch = "wasm32"))]
	pub fn queue_update() { Self::try_update(); }

	/// Queues a [microtask](https://developer.mozilla.org/en-US/docs/Web/API/Window/queueMicrotask) for update if none is set already,
	/// unlike `request_animation_frame` this will run before
	/// the next render.
	#[cfg(target_arch = "wasm32")]
	pub fn queue_update() {
		use wasm_bindgen::JsCast;
		use wasm_bindgen::closure::Closure;
		static UPDATE_QUEUED: LazyLock<Arc<std::sync::Mutex<bool>>> =
			LazyLock::new(|| Arc::new(std::sync::Mutex::new(false)));

		// Avoid scheduling multiple microtasks
		let update_flag = UPDATE_QUEUED.clone();
		{
			let mut is_queued = update_flag.lock().unwrap();
			if *is_queued {
				return;
			}
			*is_queued = true;
		}

		// Schedule a microtask that runs before the next render
		let update_flag_for_task = update_flag.clone();
		let func = Closure::once_into_js(move || {
			{
				let mut is_queued = update_flag_for_task.lock().unwrap();
				*is_queued = false;
			}
			let _ = Self::try_update();
		});
		let func_js: js_sys::Function = func.unchecked_into();
		if let Some(win) = web_sys::window() {
			let _ = win.queue_microtask(&func_js);
		}
	}
}
