//! [`Initializer`]: a value an [`App`] consumes to set itself up.
//!
//! The alternative to an extension method on [`App`], which grows it into a
//! mega type: a registration is instead a value, built by an associated fn on
//! the type or trait it registers, and handed to [`App::init`](AppInitExt::init).
//!
//! ```
//! # use beet_core::prelude::*;
//! #[derive(Default, Resource)]
//! struct Count(u32);
//!
//! /// Adds `amount` to the [`Count`].
//! fn add(amount: u32) -> impl Initializer {
//! 	move |app: &mut App| {
//! 		app.world_mut().get_resource_or_init::<Count>().0 += amount;
//! 	}
//! }
//!
//! let mut app = App::new();
//! app.init((add(1), add(2)));
//! app.world().resource::<Count>().0.xpect_eq(3);
//! ```
//!
//! Unlike a [`Plugin`] an initializer is consumed rather than borrowed, is
//! never unique (the same one may run twice), and needs no `'static`, `Send`
//! or `Sync`, so it can capture whatever the call site holds.
use crate::prelude::*;
use variadics_please::all_tuples;

/// A value an [`App`] consumes to set itself up, see the [module](self) docs.
///
/// Any `FnOnce(&mut App)` is one, and a tuple of them runs each in order.
pub trait Initializer {
	/// Set up `app`.
	fn init(self, app: &mut App);
}

impl<F: FnOnce(&mut App)> Initializer for F {
	fn init(self, app: &mut App) { self(app); }
}

macro_rules! impl_initializer_tuple {
	($(#[$meta:meta])* $(($init:ident, $name:ident)),*) => {
		$(#[$meta])*
		impl<$($init: Initializer),*> Initializer for ($($init,)*) {
			#[allow(unused_variables)]
			fn init(self, app: &mut App) {
				let ($($name,)*) = self;
				$($name.init(app);)*
			}
		}
	};
}

all_tuples!(impl_initializer_tuple, 0, 15, I, init);

/// Runs an [`Initializer`] on an [`App`], the one setup method an initializer
/// needs, rather than one extension method per registration.
#[extend::ext(name=AppInitExt)]
pub impl App {
	/// Set up this app with `initializer`, ie
	/// `app.init((Route::register_template(), UnregisteredTag::allow("Foo")))`.
	fn init(&mut self, initializer: impl Initializer) -> &mut Self {
		initializer.init(self);
		self
	}
}
