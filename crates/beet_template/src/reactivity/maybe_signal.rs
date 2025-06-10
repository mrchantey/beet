use crate::prelude::*;
use beet_common::prelude::*;
use bevy::prelude::*;

#[derive(Clone)]
pub enum MaybeSignal<T: 'static> {
	Const(T),
	Getter(Getter<T>),
}

// impl<T: 'static + Clone> Clone for MaybeSignal<T> {
// 	fn clone(&self) -> Self {
// 		match self {
// 			MaybeSignal::Const(v) => MaybeSignal::Const(v.clone()),
// 			MaybeSignal::Func(f) => MaybeSignal::Func(f.clone()),
// 		}
// 	}
// }

impl<T: 'static + Send + Clone> MaybeSignal<T> {
	pub fn value(&self) -> T {
		match self {
			MaybeSignal::Const(v) => v.clone(),
			MaybeSignal::Getter(getter) => getter.get(),
		}
	}
}

impl<T: IntoTemplateBundle<M>, M> IntoTemplateBundle<(Self, M)>
	for MaybeSignal<T>
where
	T: 'static + Send + Sync + Clone + ToString,
{
	fn into_node_bundle(self) -> impl Bundle {
		// to ensure the types match we need to create a SignalReceiver
		// for const values that will not change
		match self {
			Self::Const(val) => {
				let val = val.to_string();
				(
					TextSpan::new(val.clone()),
					SignalReceiver::new(move || val.clone()),
				)
			}
			Self::Getter(getter) => {
				// changes here should be reflected in bevy_signal.rs
				let string_getter = move || getter.get().to_string();
				(
					TextSpan::new(getter.get().to_string()),
					SignalReceiver::new(string_getter),
				)
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
