//! Installers: what an [`App`] takes in for one type or one purpose, the type
//! or value deciding what it registers, where an extension method on [`App`]
//! would grow it into a mega type.
//!
//! - [`Installer`], installed by value:
//!   `app.install(UnregisteredTag::allow("Foo"))`. Consumed rather than
//!   borrowed, never unique and needing no `'static`, `Send` or `Sync`, so it
//!   can capture whatever the call site holds. Any `FnOnce(&mut App)` is one,
//!   which a constructor returns as `impl Installer`.
//! - [`TypeInstaller`], installed by type: `app.install_type::<Route>()`. A
//!   template registers its type, type data and schema, a prescan registers
//!   itself to act before the build.
//!
//! A bevy [`Plugin`], a collection of things to happen, is both, installed
//! only when absent so a plugin pulling in what it needs composes with one
//! already holding it: `app.install(MainSchedulePlugin)`, or for a default
//! one `app.install_type::<TaskPoolPlugin>()`.
//!
//! Both take tuples, installed in order:
//! `app.install_type::<(Route, Redirect)>()`.
//!
//! ```
//! # use beet_core::prelude::*;
//! #[derive(Default, Resource)]
//! struct Count(u32);
//!
//! /// Counts every app it is installed on.
//! struct Counted;
//!
//! impl TypeInstaller for Counted {
//! 	fn install(app: &mut App) {
//! 		app.world_mut().get_resource_or_init::<Count>().0 += 1;
//! 	}
//! }
//!
//! /// Adds `amount` to the [`Count`].
//! fn increment(amount: u32) -> impl Installer {
//! 	move |app: &mut App| {
//! 		app.world_mut().get_resource_or_init::<Count>().0 += amount;
//! 	}
//! }
//!
//! let mut app = App::new();
//! app.install_type::<Counted>()
//! 	.install((increment(2), increment(3)));
//! app.world().resource::<Count>().0.xpect_eq(6);
//! ```
//!
//! A type implements an installer with the default marker, as `Counted` does;
//! the markers only tell the blanket impls apart. A bare `Fn` closure is both
//! an `FnOnce` and a bevy [`Plugin`], so `app.install(|app| ..)` cannot tell
//! which it is: return it from a constructor as `impl Installer`, which does.
use crate::prelude::*;
use variadics_please::all_tuples;

/// What an [`App`] installs by value, see the [module](self) docs.
pub trait Installer<M = ()> {
	/// Install this on `app`.
	fn install(self, app: &mut App);
}

/// What an [`App`] installs by type, see the [module](self) docs.
pub trait TypeInstaller<M = ()> {
	/// Install this type on `app`.
	fn install(app: &mut App);
}

/// The marker for a [`Plugin`] installed as an [`Installer`] or
/// [`TypeInstaller`].
pub struct PluginMarker;

impl<F: FnOnce(&mut App)> Installer for F {
	fn install(self, app: &mut App) { self(app); }
}

/// A [`Plugin`] installs itself unless one of its type is present.
impl<P: Plugin> Installer<PluginMarker> for P {
	fn install(self, app: &mut App) {
		// by type rather than `is_plugin_added`, which matches the name a
		// plugin may override
		if app.get_added_plugins::<P>().is_empty() {
			app.add_plugins(self);
		}
	}
}

/// A default [`Plugin`] installs its default, unless one of its type is
/// present.
impl<P: Plugin + Default> TypeInstaller<PluginMarker> for P {
	fn install(app: &mut App) {
		<P as Installer<PluginMarker>>::install(P::default(), app);
	}
}

macro_rules! impl_installer_tuple {
	($(#[$meta:meta])* $(($installer:ident, $marker:ident)),*) => {
		$(#[$meta])*
		impl<$($marker, $installer: Installer<$marker>),*> Installer<($($marker,)*)>
			for ($($installer,)*)
		{
			#[allow(non_snake_case)]
			fn install(self, app: &mut App) {
				let ($($installer,)*) = self;
				$($installer.install(app);)*
			}
		}

		$(#[$meta])*
		impl<$($marker, $installer: TypeInstaller<$marker>),*> TypeInstaller<($($marker,)*)>
			for ($($installer,)*)
		{
			fn install(app: &mut App) {
				$(<$installer as TypeInstaller<$marker>>::install(app);)*
			}
		}
	};
}

all_tuples!(impl_installer_tuple, 1, 15, I, M);

/// Installs an [`Installer`] or [`TypeInstaller`] on an [`App`].
///
/// The marker sits on the trait rather than each method so a call infers it:
/// `app.install_type::<Route>()` needs no turbofish beyond the type.
pub trait AppInstallExt<M> {
	/// Install `installer`, ie `app.install(UnregisteredTag::allow("Foo"))`.
	fn install(&mut self, installer: impl Installer<M>) -> &mut Self;
	/// Install `T`, ie `app.install_type::<Route>()`.
	fn install_type<T: TypeInstaller<M>>(&mut self) -> &mut Self;
}

impl<M> AppInstallExt<M> for App {
	fn install(&mut self, installer: impl Installer<M>) -> &mut Self {
		installer.install(self);
		self
	}
	fn install_type<T: TypeInstaller<M>>(&mut self) -> &mut Self {
		T::install(self);
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

	/// A plugin installed by type or by value, however often, is added once.
	#[crate::test]
	fn plugin_installs_once() {
		let mut app = App::new();
		app.install_type::<Counter>()
			.install_type::<(Counter, Counter)>()
			.install(Counter);
		app.get_added_plugins::<Counter>().len().xpect_eq(1);
	}
}
