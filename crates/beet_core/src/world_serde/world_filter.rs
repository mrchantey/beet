use crate::prelude::*;
use bevy::platform::collections::hash_set::IntoIter;
use core::any::Any;
use core::any::TypeId;

/// A filter used to control which types can be added to a [`DynamicWorld`].
///
/// This filter _can_ be used more generically to represent a filter for any type;
/// its intended usage with [`DynamicWorld`] only considers [components] and [resources].
/// Adding types that are not a component or resource has no effect with `DynamicWorld`.
///
/// [`DynamicWorld`]: crate::prelude::DynamicWorld
/// [components]: bevy::ecs::prelude::Component
/// [resources]: bevy::ecs::prelude::Resource
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub enum WorldFilter {
	/// Represents an unset filter.
	///
	/// This is the equivalent of an empty [`Denylist`] or an [`Allowlist`] containing
	/// every type, ie all types are permissible.
	///
	/// [Allowing] a type converts this filter to an `Allowlist`.
	/// [denying] a type converts this filter to a `Denylist`.
	///
	/// [`Denylist`]: WorldFilter::Denylist
	/// [`Allowlist`]: WorldFilter::Allowlist
	/// [Allowing]: WorldFilter::allow
	/// [denying]: WorldFilter::deny
	#[default]
	Unset,
	/// Contains the set of permitted types by their [`TypeId`].
	Allowlist(HashSet<TypeId>),
	/// Contains the set of prohibited types by their [`TypeId`].
	Denylist(HashSet<TypeId>),
}

impl WorldFilter {
	/// Creates a filter where all types are allowed, ie an empty [`Denylist`].
	///
	/// [`Denylist`]: WorldFilter::Denylist
	pub fn allow_all() -> Self { Self::Denylist(HashSet::default()) }

	/// Creates a filter where all types are denied, ie an empty [`Allowlist`].
	///
	/// [`Allowlist`]: WorldFilter::Allowlist
	pub fn deny_all() -> Self { Self::Allowlist(HashSet::default()) }

	/// Allow the given type, `T`.
	///
	/// If this filter is a [`Denylist`] the type is removed from the denied set.
	/// If this filter is [`Unset`] it is replaced by a new [`Allowlist`].
	///
	/// [`Denylist`]: WorldFilter::Denylist
	/// [`Unset`]: WorldFilter::Unset
	/// [`Allowlist`]: WorldFilter::Allowlist
	#[must_use]
	pub fn allow<T: Any>(self) -> Self { self.allow_by_id(TypeId::of::<T>()) }

	/// Allow the given type by [`TypeId`]. See [`allow`](Self::allow).
	#[must_use]
	pub fn allow_by_id(mut self, type_id: TypeId) -> Self {
		match &mut self {
			Self::Unset => {
				self = Self::Allowlist([type_id].into_iter().collect());
			}
			Self::Allowlist(list) => {
				list.insert(type_id);
			}
			Self::Denylist(list) => {
				list.remove(&type_id);
			}
		}
		self
	}

	/// Deny the given type, `T`.
	///
	/// If this filter is an [`Allowlist`] the type is removed from the allowed set.
	/// If this filter is [`Unset`] it is replaced by a new [`Denylist`].
	///
	/// [`Allowlist`]: WorldFilter::Allowlist
	/// [`Unset`]: WorldFilter::Unset
	/// [`Denylist`]: WorldFilter::Denylist
	#[must_use]
	pub fn deny<T: Any>(self) -> Self { self.deny_by_id(TypeId::of::<T>()) }

	/// Deny the given type by [`TypeId`]. See [`deny`](Self::deny).
	#[must_use]
	pub fn deny_by_id(mut self, type_id: TypeId) -> Self {
		match &mut self {
			Self::Unset => self = Self::Denylist([type_id].into_iter().collect()),
			Self::Allowlist(list) => {
				list.remove(&type_id);
			}
			Self::Denylist(list) => {
				list.insert(type_id);
			}
		}
		self
	}

	/// Returns true if the given type, `T`, is allowed by the filter.
	///
	/// If the filter is [`Unset`](WorldFilter::Unset) this always returns `true`.
	pub fn is_allowed<T: Any>(&self) -> bool {
		self.is_allowed_by_id(TypeId::of::<T>())
	}

	/// Returns true if the given type is allowed by the filter.
	///
	/// If the filter is [`Unset`](WorldFilter::Unset) this always returns `true`.
	pub fn is_allowed_by_id(&self, type_id: TypeId) -> bool {
		match self {
			Self::Unset => true,
			Self::Allowlist(list) => list.contains(&type_id),
			Self::Denylist(list) => !list.contains(&type_id),
		}
	}

	/// Returns true if the given type, `T`, is denied by the filter.
	///
	/// If the filter is [`Unset`](WorldFilter::Unset) this always returns `false`.
	pub fn is_denied<T: Any>(&self) -> bool {
		self.is_denied_by_id(TypeId::of::<T>())
	}

	/// Returns true if the given type is denied by the filter.
	///
	/// If the filter is [`Unset`](WorldFilter::Unset) this always returns `false`.
	pub fn is_denied_by_id(&self, type_id: TypeId) -> bool {
		!self.is_allowed_by_id(type_id)
	}

	/// Returns an iterator over the items in the filter, empty if [`Unset`](WorldFilter::Unset).
	pub fn iter(&self) -> Box<dyn ExactSizeIterator<Item = &TypeId> + '_> {
		match self {
			Self::Unset => Box::new(core::iter::empty()),
			Self::Allowlist(list) | Self::Denylist(list) => Box::new(list.iter()),
		}
	}

	/// Returns the number of items in the filter, zero if [`Unset`](WorldFilter::Unset).
	pub fn len(&self) -> usize {
		match self {
			Self::Unset => 0,
			Self::Allowlist(list) | Self::Denylist(list) => list.len(),
		}
	}

	/// Returns true if there are zero items in the filter, ie [`Unset`](WorldFilter::Unset).
	pub fn is_empty(&self) -> bool {
		match self {
			Self::Unset => true,
			Self::Allowlist(list) | Self::Denylist(list) => list.is_empty(),
		}
	}
}

impl IntoIterator for WorldFilter {
	type Item = TypeId;
	type IntoIter = IntoIter<TypeId>;

	fn into_iter(self) -> Self::IntoIter {
		match self {
			Self::Unset => Default::default(),
			Self::Allowlist(list) | Self::Denylist(list) => list.into_iter(),
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[crate::test]
	fn sets_list_type_if_none() {
		matches!(WorldFilter::Unset.allow::<i32>(), WorldFilter::Allowlist(_))
			.xpect_true();
		matches!(WorldFilter::Unset.deny::<i32>(), WorldFilter::Denylist(_))
			.xpect_true();
	}

	#[crate::test]
	fn adds_to_list() {
		let filter = WorldFilter::default().allow::<i16>().allow::<i32>();
		filter.len().xpect_eq(2);
		filter.is_allowed::<i16>().xpect_true();
		filter.is_allowed::<i32>().xpect_true();

		let filter = WorldFilter::default().deny::<i16>().deny::<i32>();
		filter.len().xpect_eq(2);
		filter.is_denied::<i16>().xpect_true();
		filter.is_denied::<i32>().xpect_true();
	}

	#[crate::test]
	fn removes_from_list() {
		let filter = WorldFilter::default()
			.allow::<i16>()
			.allow::<i32>()
			.deny::<i32>();
		filter.len().xpect_eq(1);
		filter.is_allowed::<i16>().xpect_true();
		filter.is_allowed::<i32>().xpect_false();

		let filter = WorldFilter::default()
			.deny::<i16>()
			.deny::<i32>()
			.allow::<i32>();
		filter.len().xpect_eq(1);
		filter.is_denied::<i16>().xpect_true();
		filter.is_denied::<i32>().xpect_false();
	}
}
