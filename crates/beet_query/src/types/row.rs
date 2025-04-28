use crate::prelude::*;





pub struct Row(pub Vec<Value>);

impl std::ops::Deref for Row {
	type Target = Vec<Value>;
	fn deref(&self) -> &Self::Target { &self.0 }
}
impl std::ops::DerefMut for Row {
	fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}


impl Row {
	pub fn new(values: Vec<Value>) -> Self { Self(values) }
	pub fn inner(self) -> Vec<Value> { self.0 }
	pub fn into_other<T, M>(self) -> ConvertValueResult<Vec<T>>
	where
		T: ConvertValue<M>,
	{
		self.0.into_iter().map(|v| v.into_other::<T>()).collect()
	}
}
