use super::*;
use beet_core::prelude::*;
use std::hash::Hasher;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Get)]
pub struct PropertyDef {
	/// The name of this property in css,
	/// ie `background-color`
	css_name: SmolStr,
	inherit_base: bool,
	type_tag: SmolStr,
}

impl PropertyDef {
	pub const fn new(
		css_name: SmolStr,
		inherit_base: bool,
		type_tag: SmolStr,
	) -> Self {
		Self {
			css_name,
			inherit_base,
			type_tag,
		}
	}

	pub const fn new_static<T: TypeTag>(
		css_name: &'static str,
		inherited: bool,
	) -> Self {
		Self {
			css_name: SmolStr::new_static(css_name),
			inherit_base: inherited,
			type_tag: T::TYPE_TAG,
		}
	}

	pub fn property(&self) -> Property {
		Property {
			def: self.clone(),
			inherit_override: None,
			state: None,
		}
	}
}

impl From<PropertyDef> for Property {
	fn from(def: PropertyDef) -> Self { def.property() }
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


#[derive(Debug, Deref, SetWith, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Property {
	#[deref]
	#[set_with(skip)]
	def: PropertyDef,
	inherit_override: Option<Inheritance>,
	state: Option<State>,
}

impl Property {
	pub const fn new(
		def: PropertyDef,
		inherit_override: Option<Inheritance>,
		state: Option<State>,
	) -> Self {
		Self {
			def,
			inherit_override,
			state,
		}
	}

	pub fn should_inherit(&self) -> bool {
		self.inherit_override.unwrap_or_else(|| {
			if self.def.inherit_base {
				Inheritance::Inherited
			} else {
				Inheritance::Initial
			}
		}) == Inheritance::Inherited
	}

	pub fn def(&self) -> &PropertyDef { &self.def }
}

impl std::hash::Hash for Property {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.def.hash(state);
		self.inherit_override.hash(state);
		self.state.hash(state);
	}
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Deref, DerefMut, Component)]
pub struct ResolvedPropertyMap<T = ()>(HashMap<Property, StyleValue<T>>);

impl<T> ResolvedPropertyMap<T> {
	pub fn new(map: HashMap<Property, StyleValue<T>>) -> Self { Self(map) }
}

#[derive(Debug, Default, Clone, Deref, DerefMut, Component)]
pub struct PropertyMap(HashMap<Property, Token>);

impl PropertyMap {
	pub fn new(map: HashMap<Property, Token>) -> Self { Self(map) }

	pub fn with(mut self, property: impl Into<Property>, value: Token) -> Self {
		self.0.insert(property.into(), value);
		self
	}
}

/// When merging we do allow overwriting for ordered insertion.
impl Merge for PropertyMap {
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
		let mut map = PropertyMap::default();
		let prop = props::FOREGROUND_COLOR.property();
		map.insert(prop.clone(), colors::ON_PRIMARY);
		map.get(&prop).unwrap().xpect_eq(colors::ON_PRIMARY);
	}

	#[test]
	fn property_def_tracks_type_tag() {
		props::FOREGROUND_COLOR
			.type_tag()
			.xpect_eq(&Color::TYPE_TAG);
	}
}
