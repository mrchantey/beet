use bevy::prelude::*;
use std::cell::OnceCell;
use std::sync::Arc;


/// In some cases like router it is prefereable to have a single app instance
/// for each thread to avoid the overhead of creating a new app for each request.
/// This type will get the app from a thread-local storage, or initialize it using
/// the provided constructor.
///
///
/// ## Example
///
/// ```rust
/// # use beet_core::prelude::*;
/// # use bevy::prelude::*;
///
/// #[derive(Resource)]
/// struct Foo(u32);
///
/// let pool = AppPool::new(|| {
/// 	 let mut app = App::new();
/// 	app.insert_resource(Foo(1));
/// 	app
/// });
///
/// let pool2 = pool.clone();
/// std::thread::spawn(move || {
/// 	assert_eq!(pool2.get().world().resource::<Foo>().0, 1);
/// 	// will have no effect on the main thread's app
/// 	pool2.get().insert_resource(Foo(2));
/// }).join().unwrap();
///
/// assert_eq!(pool.get().world().resource::<Foo>().0, 1);
#[derive(Clone)]
pub struct AppPool {
	/// The constructor to create a new app instance, if the instance is
	/// already set this will never be called.
	constructor: Arc<dyn 'static + Send + Sync + Fn() -> App>,
}

impl AppPool {
	pub fn new<F>(constructor: F) -> Self
	where
		F: 'static + Send + Sync + Fn() -> App,
	{
		Self {
			constructor: Arc::new(constructor),
		}
	}

	pub fn get(&self) -> ThreadLocalApp {
		let constructor = self.constructor.clone();
		ThreadLocalApp::get_or_init_with(move || (constructor)())
	}
}


/// A system for setting an app constructor, and then subsequent calls
/// to [`Self::get`] will either return the stored copy of the app
/// or create a new one using the constructor.
/// This is useful for situations  like routers where the cost of creating
/// an new app for each request is high and the
pub struct ThreadLocalApp {
	app: *mut App,
}

// TODO get peer review on this
// SAFE?: ThreadLocalApp is only ever used in a thread-local context, so this is sound.
unsafe impl Send for ThreadLocalApp {}

thread_local! {
		static APP: OnceCell<App> = OnceCell::new();
}

impl ThreadLocalApp {
	fn new(app: &App) -> Self {
		Self {
			app: app as *const App as *mut App,
		}
	}

	/// Sets the constructor for the app.
	/// This should be called once before any calls to [`Self::get`].
	pub fn get_or_init_with<F>(constructor: F) -> Self
	where
		F: 'static + Send + Sync + Fn() -> App,
	{
		APP.with(|cell| {
			let app_ref = cell.get_or_init(|| constructor());
			ThreadLocalApp::new(app_ref)
		})
	}
	/// Gets a thread-local instance of the app
	///
	/// # Panics
	/// If the constructor has not been set before calling this method.
	pub fn get() -> Option<Self> {
		APP.with(|cell| cell.get().map(|app| ThreadLocalApp::new(app)))
	}
}

impl std::ops::Deref for ThreadLocalApp {
	type Target = App;
	fn deref(&self) -> &Self::Target { unsafe { &*self.app } }
}

impl std::ops::DerefMut for ThreadLocalApp {
	fn deref_mut(&mut self) -> &mut Self::Target { unsafe { &mut *self.app } }
}
#[cfg(test)]
mod tests {
	use super::*;
	use sweet::prelude::*;

	#[derive(Resource)]
	struct Foo(u32);

	// only use a single test so it wont be run in parallel
	#[test]
	fn works() {
		let pool = AppPool::new(|| {
			let mut app = App::new();
			app.insert_resource(Foo(1));
			app
		});


		// Main thread
		let mut main_app = pool.get();
		main_app.world().resource::<Foo>().0.xpect().to_be(1);

		// Change value in main thread
		main_app.world_mut().insert_resource(Foo(2));
		main_app.world().resource::<Foo>().0.xpect().to_be(2);

		// Spawn a thread and check isolation
		let pool2 = pool.clone();
		#[cfg(not(target_arch = "wasm32"))]
		std::thread::spawn(move || {
			let mut thread_app = pool2.get();
			// Should be the original value, not affected by main thread
			thread_app.world().resource::<Foo>().0.xpect().to_be(1);
			thread_app.world_mut().insert_resource(Foo(3));
			thread_app.world().resource::<Foo>().0.xpect().to_be(3);
		})
		.join()
		.unwrap();

		// Main thread value should remain unchanged
		main_app.world().resource::<Foo>().0.xpect().to_be(2);
	}
}
