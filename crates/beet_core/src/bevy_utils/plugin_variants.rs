//! Plugin variants: bevy's [`Plugin`] is the borrowed one, built through
//! `&self`, unique per type and `'static + Send + Sync`. Two more cover what
//! would otherwise be an extension method on [`App`], growing it into a mega
//! type: the type or value decides what it registers, the call site just adds
//! it.
//!
//! - [`TypePlugin`], added by type: `app.add_type::<Route>()`. A template
//!   registers its type, type data and schema, a default [`Plugin`] adds
//!   itself unless present.
//! - [`OwnedPlugin`], added by value: `app.add(UnregisteredTag::allow("Foo"))`.
//!   Consumed rather than borrowed, never unique and needing no `'static`,
//!   `Send` or `Sync`, so it can capture whatever the call site holds. Any
//!   `FnOnce(&mut App)` is one.
//!
//! Both take tuples, run in order: `app.add_type::<(Route, Redirect)>()`.
//!
//! ```
//! # use beet_core::prelude::*;
//! #[derive(Default, Resource)]
//! struct Count(u32);
//!
//! /// Counts every app it is added to.
//! struct Counted;
//!
//! impl TypePlugin for Counted {
//! 	fn build(app: &mut App) {
//! 		app.world_mut().get_resource_or_init::<Count>().0 += 1;
//! 	}
//! }
//!
//! /// Adds `amount` to the [`Count`].
//! fn increment(amount: u32) -> impl FnOnce(&mut App) {
//! 	move |app: &mut App| {
//! 		app.world_mut().get_resource_or_init::<Count>().0 += amount;
//! 	}
//! }
//!
//! let mut app = App::new();
//! app.add_type::<Counted>().add((increment(2), increment(3)));
//! app.world().resource::<Count>().0.xpect_eq(6);
//! ```
//!
//! A type implements a variant directly with the default marker, as `Counted`
//! does; the markers only tell a blanket impl apart from another.
use crate::prelude::*;
use variadics_please::all_tuples;

/// A plugin added by type, deciding for itself what it registers, see the
/// [module](self) docs.
pub trait TypePlugin<M = ()> {
	/// Register this type on `app`.
	fn build(app: &mut App);
}

/// A plugin consumed by value, see the [module](self) docs.
pub trait OwnedPlugin<M = ()> {
	/// Set up `app`.
	fn build(self, app: &mut App);
}

/// The [`TypePlugin`] marker for a default [`Plugin`].
pub struct PluginMarker;

/// The [`OwnedPlugin`] marker for a `FnOnce(&mut App)`.
pub struct FuncMarker;

/// A default [`Plugin`] adds itself unless present, so a plugin pulling in
/// what it needs composes with one already holding it.
impl<T: Plugin + Default> TypePlugin<PluginMarker> for T {
	fn build(app: &mut App) { IfAbsent(T::default()).build(app); }
}

impl<F: FnOnce(&mut App)> OwnedPlugin<FuncMarker> for F {
	fn build(self, app: &mut App) { self(app); }
}

/// Adds the wrapped [`Plugin`] unless one of its type is present, the
/// [`OwnedPlugin`] form of a plugin with no [`Default`]:
/// `app.add(IfAbsent(MainSchedulePlugin))`.
pub struct IfAbsent<P>(pub P);

impl<P: Plugin> OwnedPlugin for IfAbsent<P> {
	fn build(self, app: &mut App) {
		// by type rather than `is_plugin_added`, which matches the name a
		// plugin may override
		if app.get_added_plugins::<P>().is_empty() {
			app.add_plugins(self.0);
		}
	}
}

macro_rules! impl_type_plugin_tuple {
	($(#[$meta:meta])* $(($plugin:ident, $marker:ident)),*) => {
		$(#[$meta])*
		impl<$($marker, $plugin: TypePlugin<$marker>),*> TypePlugin<($($marker,)*)>
			for ($($plugin,)*)
		{
			fn build(app: &mut App) { $($plugin::build(app);)* }
		}
	};
}

all_tuples!(impl_type_plugin_tuple, 1, 15, P, M);

macro_rules! impl_owned_plugin_tuple {
	($(#[$meta:meta])* $(($plugin:ident, $marker:ident)),*) => {
		$(#[$meta])*
		impl<$($marker, $plugin: OwnedPlugin<$marker>),*> OwnedPlugin<($($marker,)*)>
			for ($($plugin,)*)
		{
			#[allow(non_snake_case)]
			fn build(self, app: &mut App) {
				let ($($plugin,)*) = self;
				$($plugin.build(app);)*
			}
		}
	};
}

all_tuples!(impl_owned_plugin_tuple, 1, 15, P, M);

/// Adds the [`TypePlugin`] and [`OwnedPlugin`] variants to an [`App`].
///
/// The marker sits on the trait rather than each method so a call infers it:
/// `app.add_type::<Route>()` needs no turbofish beyond the type.
pub trait AppPluginVariantsExt<M> {
	/// Add the [`OwnedPlugin`] `plugin`, ie `app.add(UnregisteredTag::allow("Foo"))`.
	fn add(&mut self, plugin: impl OwnedPlugin<M>) -> &mut Self;
	/// Add the [`TypePlugin`] `T`, ie `app.add_type::<Route>()`.
	fn add_type<T: TypePlugin<M>>(&mut self) -> &mut Self;
}

impl<M> AppPluginVariantsExt<M> for App {
	fn add(&mut self, plugin: impl OwnedPlugin<M>) -> &mut Self {
		plugin.build(self);
		self
	}
	fn add_type<T: TypePlugin<M>>(&mut self) -> &mut Self {
		T::build(self);
		self
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[derive(Default)]
	struct Counter;

	impl Plugin for Counter {
		fn build(&self, _app: &mut App) {}
	}

	/// A default plugin added by type twice is built once.
	#[crate::test]
	fn default_plugin_once() {
		let mut app = App::new();
		app.add_type::<Counter>()
			.add_type::<(Counter, Counter)>()
			.add(IfAbsent(Counter));
		app.get_added_plugins::<Counter>().len().xpect_eq(1);
	}
}
