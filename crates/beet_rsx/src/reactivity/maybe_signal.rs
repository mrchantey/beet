use crate::prelude::*;
use beet_core::prelude::BundleExt;
use beet_core::prelude::*;
use bevy::prelude::*;

#[derive(Clone)]
pub enum MaybeSignal<T: 'static> {
	Const(T),
	Getter(Getter<T>),
}

impl<T: 'static + Send + Clone> MaybeSignal<T> {
	pub fn value(&self) -> T {
		match self {
			MaybeSignal::Const(v) => v.clone(),
			MaybeSignal::Getter(getter) => getter.get(),
		}
	}
}


impl<T, M1, M2> IntoTemplateBundle<(Self, M1, M2)> for MaybeSignal<T>
where
	T: IntoTemplateBundle<M1>,
	Getter<T>: IntoTemplateBundle<M2>,
{
	fn into_template_bundle(self) -> impl Bundle {
		match self {
			Self::Const(val) => val.into_template_bundle().any_bundle(),
			Self::Getter(getter) => {
				getter.into_template_bundle().any_bundle()
			}
		}
	}
}


impl<T: 'static + Send + Clone + std::fmt::Debug> std::fmt::Debug
	for MaybeSignal<T>
{
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			MaybeSignal::Const(val) => write!(f, "Const({:?})", val),
			MaybeSignal::Getter(getter) => {
				write!(f, "Getter({:?})", getter.get())
			}
		}
	}
}

impl<T: 'static + Send + Clone + ToString> ToString for MaybeSignal<T> {
	fn to_string(&self) -> String { self.value().to_string() }
}


pub trait IntoMaybeSignal<T, M> {
	fn into_maybe_signal(self) -> MaybeSignal<T>;
}
pub struct IntoIntoMaybeSignalMarker;

impl<T, V: Into<T>> IntoMaybeSignal<T, IntoIntoMaybeSignalMarker> for V {
	fn into_maybe_signal(self) -> MaybeSignal<T> {
		MaybeSignal::Const(self.into())
	}
}

pub struct GetterIntoMaybeSignalMarker;
impl<T: 'static + Send + Clone> IntoMaybeSignal<T, GetterIntoMaybeSignalMarker>
	for Getter<T>
{
	fn into_maybe_signal(self) -> MaybeSignal<T> { MaybeSignal::Getter(self) }
}
