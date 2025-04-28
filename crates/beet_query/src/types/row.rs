use crate::prelude::*;

#[derive(Debug, thiserror::Error)]
pub enum RowsError {
	#[error("Row Length Mismatch: expected: {expected}, received: {received}")]
	LengthMismatch { expected: usize, received: usize },
}


#[derive(Debug, Default, Clone, PartialEq, PartialOrd)]
pub struct Rows(pub Vec<Row>);

impl std::fmt::Display for Rows {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		if self.0.is_empty() {
			return write!(f, "[]");
		}
		
		writeln!(f, "[")?;
		for (i, row) in self.0.iter().enumerate() {
			write!(f, "  {}", row)?;
			if i < self.0.len() - 1 {
				writeln!(f, ",")?;
			} else {
				writeln!(f)?;
			}
		}
		write!(f, "]")
	}
}


impl std::ops::Deref for Rows {
	type Target = Vec<Row>;
	fn deref(&self) -> &Self::Target { &self.0 }
}
impl std::ops::DerefMut for Rows {
	fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}


impl Rows {
	pub fn new(values: Vec<Row>) -> Self { Self(values) }
	pub fn inner(self) -> Vec<Row> { self.0 }


	pub fn exactly_one(self) -> Result<Row, RowsError> {
		if self.0.len() != 1 {
			return Err(RowsError::LengthMismatch {
				expected: 1,
				received: self.0.len(),
			});
		}
		Ok(self.0.into_iter().next().unwrap())
	}
}
/// Convert a [`Rows`] into another type by specifying the type
/// but not the marker.
pub trait RowsIntoOther<M> {
	fn into_other<T>(self) -> ConvertValueResult<Vec<Vec<T>>>
	where
		T: ConvertValue<M>;
}

impl<M> RowsIntoOther<M> for Rows {
	fn into_other<T>(self) -> ConvertValueResult<Vec<Vec<T>>>
	where
		T: ConvertValue<M>,
	{
		self.0.into_iter().map(|v| v.into_other::<T>()).collect()
	}
}


#[derive(Debug, Default, Clone, PartialEq, PartialOrd)]
pub struct Row(pub Vec<Value>);

impl std::ops::Deref for Row {
	type Target = Vec<Value>;
	fn deref(&self) -> &Self::Target { &self.0 }
}
impl std::ops::DerefMut for Row {
	fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}

impl std::fmt::Display for Row {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"[{}]",
			self.0
				.iter()
				.map(|v| v.to_string())
				.collect::<Vec<_>>()
				.join(", ")
		)
	}
}


impl Row {
	pub fn new(values: Vec<Value>) -> Self { Self(values) }
	pub fn inner(self) -> Vec<Value> { self.0 }
}


/// Convert a [`Row`] into another type by specifying the type
/// but not the marker.
pub trait RowIntoOther<M> {
	fn into_other<T>(self) -> ConvertValueResult<Vec<T>>
	where
		T: ConvertValue<M>;
}

impl<M> RowIntoOther<M> for Row {
	fn into_other<T>(self) -> ConvertValueResult<Vec<T>>
	where
		T: ConvertValue<M>,
	{
		self.0.into_iter().map(|v| v.into_other::<T>()).collect()
	}
}
