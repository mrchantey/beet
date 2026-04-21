use super::*;
use beet_core::prelude::*;
use std::hash::Hasher;


#[derive(Get)]
pub struct PropertyDef<T> {
	/// The name of this property in css,
	/// ie `background-color`
	css_name: SmolStr,
	inherit_base: bool,
	phantom: PhantomData<T>,
}

impl<T> PropertyDef<T> {
	pub const fn new_static(css_name: &'static str, inherited: bool) -> Self {
		Self {
			css_name: SmolStr::new_static(css_name),
			inherit_base: inherited,
			phantom: PhantomData::<T>,
		}
	}
	pub fn property(&self) -> Property<T> {
		Property {
			def: self.clone(),
			inherit_override: None,
			state: None,
		}
	}
}

impl<T> From<PropertyDef<T>> for Property<T> {
	fn from(def: PropertyDef<T>) -> Self { def.property() }
}

impl<T> std::fmt::Debug for PropertyDef<T> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Property")
			.field("css_name", &self.css_name)
			.field("inherited", &self.inherit_base)
			.field("category", &Token::<T>::category())
			.finish()
	}
}
impl<T> Clone for PropertyDef<T> {
	fn clone(&self) -> Self {
		Self {
			css_name: self.css_name.clone(),
			inherit_base: self.inherit_base.clone(),
			phantom: PhantomData,
		}
	}
}

impl<T> PartialEq for PropertyDef<T> {
	fn eq(&self, other: &Self) -> bool { self.css_name == other.css_name }
}

impl<T> Eq for PropertyDef<T> {}
impl<T> PartialOrd for PropertyDef<T> {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		Some(self.cmp(other))
	}
}
impl<T> Ord for PropertyDef<T> {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		self.css_name.cmp(&other.css_name)
	}
}

impl<T> std::hash::Hash for PropertyDef<T> {
	fn hash<H: Hasher>(&self, state: &mut H) { self.css_name.hash(state); }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum State {
	Hovered,
	Focused,
	Pressed,
	Dragged,
	Disabled,
	Selected,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Inheritance {
	Inherited,
	Initial,
	Unset,
}


#[macro_export]
macro_rules! property {
	($kind:ident, $name:ident, $token:expr, $state:expr) => {
		pub const $name: Property<$kind> = Property::new_static($token, $state);
	};
}

#[derive(Debug, Deref, SetWith)]
pub struct Property<T> {
	#[deref]
	#[set_with(skip)]
	def: PropertyDef<T>,
	inherit_override: Option<Inheritance>,
	state: Option<State>,
}

impl<T> Property<T> {
	pub fn should_inherit(&self) -> bool {
		self.inherit_override.unwrap_or_else(|| {
			if self.def.inherit_base {
				Inheritance::Inherited
			} else {
				Inheritance::Initial
			}
		}) == Inheritance::Inherited
	}
}

impl<T> PartialEq for Property<T> {
	fn eq(&self, other: &Self) -> bool {
		self.def == other.def
			&& self.inherit_override == other.inherit_override
			&& self.state == other.state
	}
}

impl<T> Clone for Property<T> {
	fn clone(&self) -> Self {
		Self {
			def: self.def.clone(),
			inherit_override: self.inherit_override.clone(),
			state: self.state.clone(),
		}
	}
}
impl<T> Eq for Property<T> {}

impl<T> PartialOrd for Property<T> {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		Some(self.cmp(other))
	}
}

impl<T> Ord for Property<T> {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		match self.def.cmp(&other.def) {
			std::cmp::Ordering::Equal => {
				match self.inherit_override.cmp(&other.inherit_override) {
					std::cmp::Ordering::Equal => self.state.cmp(&other.state),
					other => other,
				}
			}
			other => other,
		}
	}
}

impl<T> std::hash::Hash for Property<T> {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.def.hash(state);
		self.inherit_override.hash(state);
		self.state.hash(state);
	}
}





#[derive(Debug, Default, Clone, PartialEq, Eq, Deref, DerefMut, Component)]
pub struct ResolvedPropertyMap<T>(HashMap<Property<T>, T>);
impl<T> ResolvedPropertyMap<T> {
	pub fn new(map: HashMap<Property<T>, T>) -> Self { Self(map) }
}


#[derive(Debug, Clone, Deref, DerefMut, Component)]
pub struct PropertyMap<T>(HashMap<Property<T>, Token<T>>);


impl<T> Default for PropertyMap<T> {
	fn default() -> Self { Self::new() }
}
impl<T> PropertyMap<T> {
	pub fn new() -> Self { Self(HashMap::new()) }
	pub fn with(
		mut self,
		property: impl Into<Property<T>>,
		value: Token<T>,
	) -> Self {
		self.0.insert(property.into(), value);
		self
	}
}

/// When merging we do allow overwriting for ordered insertion.
impl Merge for PropertyMap<Color> {
	fn merge(&mut self, other: Self) -> Result {
		for (key, value) in other.0 {
			self.0.insert(key, value);
		}
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn property_map() {
		let mut map = PropertyMap::<Color>::default();
		let prop = props::FOREGROUND_COLOR.property();
		map.insert(prop.clone(), colors::ON_PRIMARY);
		map.get(&prop).unwrap().xpect_eq(colors::ON_PRIMARY);
	}
}
