/// Basically a `FnOnce` trait, but not nightly and a little less awkward to implement.
pub trait Pipeline<In, Out = In> {
	/// Consume self and apply to the target
	fn apply(self, value: In) -> Out;
}

impl<F, In, Out> Pipeline<In, Out> for F
where
	F: FnOnce(In) -> Out,
{
	fn apply(self, value: In) -> Out { self(value) }
}


/// Utilities for method-chaining on any type.
/// Very similar in its goals to [`tap`](https://crates.io/crates/tap)
pub trait PipelineTarget: Sized {
	/// its like map but for any type
	fn xmap<O>(self, func: impl FnOnce(Self) -> O) -> O { func(self) }
	/// its like inpsect but for any type
	fn xtap(mut self, func: impl FnOnce(&mut Self)) -> Self {
		func(&mut self);
		self
	}
	fn xdebug(self) -> Self
	where
		Self: std::fmt::Debug,
	{
		println!("{:?}", self);
		self
	}
	fn xdisplay(self) -> Self
	where
		Self: std::fmt::Display,
	{
		println!("{}", self);
		self
	}
	fn xtap_mut(&mut self, func: impl FnOnce(&mut Self)) -> &mut Self {
		func(self);
		self
	}
	/// its like map but for our pipeline trait
	fn xpipe<P: Pipeline<Self, O>, O>(self, pipeline: P) -> O {
		pipeline.apply(self)
	}

	fn xref(&self) -> &Self { self }
	fn xok<E>(self) -> Result<Self, E> { Ok(self) }
	fn xsome(self) -> Option<Self> { Some(self) }

	fn xinto<T: From<Self>>(self) -> T { T::from(self) }
}
impl<T: Sized> PipelineTarget for T {}


/// Utilities for method-chaining on any type.
/// Very similar in its goals to [`tap`](https://crates.io/crates/tap)
pub trait PipelineTargetIter<T>: Sized + IntoIterator<Item = T> {
	/// its [`IntoIterator::into_iter().map(func).collect()`]
	fn xmap_each<O>(self, func: impl FnMut(T) -> O) -> Vec<O> {
		self.into_iter().map(func).collect()
	}
	/// its [`IntoIterator::into_iter().filter_map(func).collect()`]
	/// but flattens the results.
	fn xtry_filter_map<O, E>(
		self,
		mut func: impl FnMut(T) -> Result<Option<O>, E>,
	) -> Result<Vec<O>, E> {
		let mut out = Vec::new();
		for item in self.into_iter() {
			match (func)(item) {
				Ok(Some(o)) => out.push(o),
				Ok(None) => {}
				Err(e) => return Err(e),
			}
		}
		Ok(out)
	}
}

impl<T: Sized, I: IntoIterator<Item = T>> PipelineTargetIter<T> for I {}

pub trait PipelineTargetVec<T> {
	/// its [`Vec::extend`] but returns [`Self`]
	fn xtend<I: IntoIterator<Item = T>>(self, iter: I) -> Self;
}

impl<T, T2> PipelineTargetVec<T> for T2
where
	T2: AsMut<Vec<T>>,
{
	fn xtend<I: IntoIterator<Item = T>>(mut self, iter: I) -> Self {
		self.as_mut().extend(iter);
		self
	}
}

pub trait PipelineTargetString {
	/// its [`String::push_str`] but returns [`Self`]
	fn xtend(self, item: impl AsRef<str>) -> Self;
}

impl PipelineTargetString for String {
	fn xtend(mut self, item: impl AsRef<str>) -> Self {
		self.push_str(item.as_ref());
		self
	}
}
