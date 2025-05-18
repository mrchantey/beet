use crate::prelude::*;


pub type FileSpanOf<T> = MetaOf<FileSpan, T>;
pub type RustyTrackerOf<T> = MetaOf<RustyTracker, T>;

/// When a meta type, ie [`FileSpan`] or [`RustyTracker`] is used to represent
/// a part of an entity instead of the entire entity.
///
/// ## Example

/// The following `hidden` attribute contains three [`FileSpan`] types:
/// ```ignore
/// # use beet_common::prelude::*;
/// rsx!{ <span hidden=true /> }
/// ```
/// - [`FileSpan`] to represent the key value pair
/// - [`FileSpanOf<AttributeKey>`] for the key
/// - [`FileSpanOf<AttributeValue>`] for the value
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct MetaOf<C, T> {
	value: T,
	phantom: std::marker::PhantomData<C>,
}
impl<C, T> std::ops::Deref for MetaOf<C, T> {
	type Target = T;
	fn deref(&self) -> &Self::Target { &self.value }
}

impl<C, T> MetaOf<C, T> {
	pub fn new(value: T) -> Self {
		Self {
			value,
			phantom: std::marker::PhantomData,
		}
	}
}
