use bevy::prelude::*;
use std::cell::RefCell;
use std::sync::Arc;
use std::sync::RwLock;


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
	/// Incremented each time an app is constructed
	num_constructed: Arc<RwLock<usize>>,
	/// Incremented each time an app is returned to the pool
	num_returned: Arc<RwLock<usize>>,
}

thread_local! {
		static POOL: RefCell<Vec<World>> = RefCell::new(Vec::new());
}


impl AppPool {
	pub fn new<F>(constructor: F) -> Self
	where
		F: 'static + Send + Sync + Fn() -> App,
	{
		Self {
			constructor: Arc::new(constructor),
			num_constructed: Arc::new(RwLock::new(0)),
			num_returned: Arc::new(RwLock::new(0)),
		}
	}

	pub fn num_constructed(&self) -> usize {
		*self.num_constructed.read().unwrap()
	}

	
	pub fn num_returned(&self) -> usize { *self.num_returned.read().unwrap() }

	/// just run the constructor
	pub fn pop(&self) -> PooledWorld {
		PooledWorld {
			inner: std::mem::take((self.constructor)().world_mut()),
			num_returned: self.num_returned.clone(),
		}
	}
	
	// /// Take an app from the pool, or create a new one using the constructor.
	// /// This should be returned to the pool using [`Self::push`] when done.
	// pub fn pop(&self) -> PooledWorld {
	// 	let Self {
	// 		num_constructed,
	// 		constructor,
	// 		num_returned,
	// 	} = self.clone();
	// 	let inner = POOL.with(move |pool| {
	// 		pool.borrow_mut().pop().unwrap_or_else(move || {
	// 			*num_constructed.write().unwrap() += 1;
	// 			std::mem::take(constructor().world_mut())
	// 		})
	// 	});
	// 	PooledWorld {
	// 		inner,
	// 		num_returned,
	// 	}
	// }
}

/// An [`App`] that is automatically returned to the [`AppPool`] when dropped.
#[derive(Default)]
pub struct PooledWorld {
	inner: World,
	num_returned: Arc<RwLock<usize>>,
}
impl PooledWorld {
	pub fn inner_mut(&mut self) -> &mut World { &mut self.inner }
}

impl std::ops::Deref for PooledWorld {
	type Target = World;

	fn deref(&self) -> &Self::Target { &self.inner }
}
impl std::ops::DerefMut for PooledWorld {
	fn deref_mut(&mut self) -> &mut Self::Target { &mut self.inner }
}

impl Drop for PooledWorld {
	fn drop(&mut self) {
		*self.num_returned.write().unwrap() += 1;
		let app = std::mem::take(&mut self.inner);
		POOL.with(move |pool| {
			pool.borrow_mut().push(app);
		});
	}
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
		let mut main_app = pool.pop();
		main_app.resource::<Foo>().0.xpect().to_be(1);

		// Change value in main thread
		main_app.insert_resource(Foo(2));
		main_app.resource::<Foo>().0.xpect().to_be(2);

		// Spawn a thread and check isolation
		let pool2 = pool.clone();
		#[cfg(not(target_arch = "wasm32"))]
		std::thread::spawn(move || {
			let mut thread_app = pool2.pop();
			// Should be the original value, not affected by main thread
			thread_app.resource::<Foo>().0.xpect().to_be(1);
			thread_app.insert_resource(Foo(3));
			thread_app.resource::<Foo>().0.xpect().to_be(3);
		})
		.join()
		.unwrap();

		// Main thread value should remain unchanged
		main_app.resource::<Foo>().0.xpect().to_be(2);

		// One for main thread, one for the spawned thread
		pool.num_constructed().xpect().to_be(2);
		pool.pop();
		pool.num_constructed().xpect().to_be(3);

		pool.num_returned().xpect().to_be(2);
		drop(main_app);
		pool.num_returned().xpect().to_be(3);
	}
}
