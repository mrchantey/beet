use beet_core::prelude::*;
use bevy::prelude::*;

pub struct MaybeSignal<T: 'static> {
	get_value: Box<dyn 'static + Send + Sync + Fn() -> T>,
	get_bundle: Box<dyn 'static + Send + Sync + Fn() -> OnSpawnBoxed>,
}

impl<T: 'static> MaybeSignal<T> {
	pub fn new(
		get_value: impl 'static + Send + Sync + Fn() -> T,
		get_bundle: impl 'static + Send + Sync + Fn() -> OnSpawnBoxed,
	) -> Self {
		MaybeSignal {
			get_value: Box::new(get_value),
			get_bundle: Box::new(get_bundle),
		}
	}

	/// Get the inner value, either by cloning the const
	/// or calling the func
	pub fn value(&self) -> T { (self.get_value)() }
}

impl<T, M> IntoBundle<(Self, M)> for MaybeSignal<T>
where
	T: 'static + Send + Sync + IntoBundle<M>,
{
	fn into_bundle(self) -> impl Bundle { (self.get_bundle)() }
}

impl<T: 'static + Send + Clone + std::fmt::Debug> std::fmt::Debug
	for MaybeSignal<T>
{
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "MaybeSignal({:?})", self.value())
	}
}

impl<T: 'static + Send + Clone + std::fmt::Display> std::fmt::Display
	for MaybeSignal<T>
{
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		self.value().fmt(f)
	}
}

pub trait IntoMaybeSignal<T, M> {
	fn into_maybe_signal(self) -> MaybeSignal<T>;
}
pub struct IntoIntoMaybeSignalMarker;

impl<T, M, U> IntoMaybeSignal<T, (M, IntoIntoMaybeSignalMarker)> for U
where
	U: Into<T>,
	T: 'static + Send + Sync + Clone + IntoBundle<M>,
{
	fn into_maybe_signal(self) -> MaybeSignal<T> {
		let val = self.into();
		let val2 = val.clone();
		MaybeSignal::new(
			move || val.clone(),
			move || OnSpawnBoxed::insert(val2.clone().into_bundle()),
		)
	}
}

pub struct FuncIntoMaybeSignalMarker;
impl<T, M, F> IntoMaybeSignal<T, (M, FuncIntoMaybeSignalMarker)> for F
where
	F: 'static + Send + Sync + Clone + IntoBundle<M> + FnOnce() -> T,
{
	fn into_maybe_signal(self) -> MaybeSignal<T> {
		let self2 = self.clone();

		MaybeSignal::new(
			move || (self.clone())(),
			move || OnSpawnBoxed::insert((self2.clone()).into_bundle()),
		)
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_utils::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let (get, set) = signal("foo");

		let sig: MaybeSignal<&str> = get.into_maybe_signal();
		// let sig = get.into_maybe_signal();
		sig.value().xpect().to_be("foo");
		set("bar");
		sig.value().xpect().to_be("bar");
	}
}
