use beet_core::prelude::*;
use bevy::prelude::*;


/// This type may be used to represent:
/// - a constant value `T`
/// - a [`Getter<T>`]
/// - a function that returns `T`, ie a derived signal
pub struct DerivedGetter<T: 'static> {
	get_value: Box<dyn 'static + Send + Sync + Fn() -> T>,
	get_bundle: Box<dyn 'static + Send + Sync + Fn() -> OnSpawnBoxed>,
}

impl<T: 'static> DerivedGetter<T> {
	pub fn new(
		get_value: impl 'static + Send + Sync + Fn() -> T,
		get_bundle: impl 'static + Send + Sync + Fn() -> OnSpawnBoxed,
	) -> Self {
		DerivedGetter {
			get_value: Box::new(get_value),
			get_bundle: Box::new(get_bundle),
		}
	}

	/// Get the inner value, either by cloning the const
	/// or calling the func
	pub fn value(&self) -> T { (self.get_value)() }
}

impl<T, M> IntoBundle<(Self, M)> for DerivedGetter<T>
where
	T: 'static + Send + Sync + IntoBundle<M>,
{
	fn into_bundle(self) -> impl Bundle { (self.get_bundle)() }
}

impl<T: 'static + Send + Clone + std::fmt::Debug> std::fmt::Debug
	for DerivedGetter<T>
{
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "DerivedGetter({:?})", self.value())
	}
}

impl<T: 'static + Send + Clone + std::fmt::Display> std::fmt::Display
	for DerivedGetter<T>
{
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		self.value().fmt(f)
	}
}

pub trait IntoDerivedGetter<T, M> {
	fn into_derived_getter(self) -> DerivedGetter<T>;
}
pub struct IntoIntoDerivedGetterMarker;

impl<T, M, U> IntoDerivedGetter<T, (M, IntoIntoDerivedGetterMarker)> for U
where
	U: Into<T>,
	T: 'static + Send + Sync + Clone + IntoBundle<M>,
{
	fn into_derived_getter(self) -> DerivedGetter<T> {
		let val = self.into();
		let val2 = val.clone();
		DerivedGetter::new(
			move || val.clone(),
			move || OnSpawnBoxed::insert(val2.clone().into_bundle()),
		)
	}
}

pub struct FuncIntoDerivedGetterMarker;
impl<T, M, F> IntoDerivedGetter<T, (M, FuncIntoDerivedGetterMarker)> for F
where
	F: 'static + Send + Sync + Clone + IntoBundle<M> + FnOnce() -> T,
{
	fn into_derived_getter(self) -> DerivedGetter<T> {
		let self2 = self.clone();

		DerivedGetter::new(
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

		let sig: DerivedGetter<&str> = get.into_derived_getter();
		// let sig = get.into_derived_getter();
		sig.value().xpect().to_be("foo");
		set("bar");
		sig.value().xpect().to_be("bar");
	}
}
