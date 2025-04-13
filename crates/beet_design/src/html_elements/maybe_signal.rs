/// A type that can be either a constant value or a function that returns a value.
pub enum MaybeSignal<T> {
	Const(T),
	Func(Box<dyn 'static + Send + Sync + Fn() -> T>),
}

impl<T: ToString> ToString for MaybeSignal<T> {
	fn to_string(&self) -> String {
		match self {
			MaybeSignal::Const(v) => v.to_string(),
			MaybeSignal::Func(f) => f().to_string(),
		}
	}
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

pub struct FuncIntoMaybeSignalMarker;
impl<F, T, T2> IntoMaybeSignal<T, FuncIntoMaybeSignalMarker> for F
where
	F: 'static + Send + Sync + Fn() -> T2,
	T2: Into<T>,
{
	fn into_maybe_signal(self) -> MaybeSignal<T> {
		MaybeSignal::Func(Box::new(move || self().into()))
	}
}
