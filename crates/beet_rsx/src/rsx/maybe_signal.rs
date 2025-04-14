/// A type that can be either a constant value or a function that returns a value.


pub trait Getter<T>: 'static + Send + Sync + Fn() -> T {
	fn clone_box(&self) -> Box<dyn Getter<T>>;
}
impl<T, F> Getter<T> for F
where
	F: 'static + Send + Sync + Clone + Fn() -> T,
{
	fn clone_box(&self) -> Box<dyn Getter<T>> { Box::new(self.clone()) }
}
impl<T: 'static> Clone for Box<dyn Getter<T>> {
	fn clone(&self) -> Self { (**self).clone_box() }
}

#[derive(Clone)]
pub enum MaybeSignal<T: 'static> {
	Const(T),
	Func(Box<dyn Getter<T>>),
}

// impl<T: 'static + Clone> Clone for MaybeSignal<T> {
// 	fn clone(&self) -> Self {
// 		match self {
// 			MaybeSignal::Const(v) => MaybeSignal::Const(v.clone()),
// 			MaybeSignal::Func(f) => MaybeSignal::Func(f.clone()),
// 		}
// 	}
// }

impl<T: Clone> MaybeSignal<T> {
	pub fn value(&self) -> T {
		match self {
			MaybeSignal::Const(v) => v.clone(),
			MaybeSignal::Func(f) => f(),
		}
	}
}

impl<T: std::fmt::Debug> std::fmt::Debug for MaybeSignal<T> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			MaybeSignal::Const(v) => write!(f, "Const({:?})", v),
			MaybeSignal::Func(v) => write!(f, "Func({:?})", v()),
		}
	}
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
	F: 'static + Send + Sync + Clone + Fn() -> T2 + Getter<T2>,
	T2: Into<T>,
{
	fn into_maybe_signal(self) -> MaybeSignal<T> {
		MaybeSignal::Func(Box::new(move || self().into()))
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[test]
	fn clones_without_recursion() {
		let val: Box<dyn Getter<usize>> = Box::new(|| 5);
		let _val2 = val.clone();
	}
}
