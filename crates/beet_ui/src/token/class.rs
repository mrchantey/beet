use crate::prelude::*;
use beet_core::prelude::*;
use bevy::reflect::Typed;


/// A class name assigned to an element entity.
///
/// In addition to the `class` attribute, classes may be stored directly on an
/// element via the [`Classes`] component, allowing for efficient and ergonomic
/// runtime class addition and removal.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ClassName {
	/// An arbitrary string for a class name.
	String(SmolStr),
	/// A class derived from a source location, ensuring uniqueness and
	/// stability across spawns of the same callsite.
	Inline { file: SmolStr, line: u32, col: u32 },
}

impl ClassName {
	pub fn string(name: impl Into<SmolStr>) -> Self {
		Self::String(name.into())
	}

	/// Create an inline class from the caller's source location.
	#[track_caller]
	pub fn new_inline() -> Self {
		let location = core::panic::Location::caller();
		Self::Inline {
			file: location.file().into(),
			line: location.line(),
			col: location.column(),
		}
	}

	/// The string used when matching against [`Selector::Class`].
	pub fn as_selector(&self) -> SmolStr {
		match self {
			Self::String(s) => s.clone(),
			Self::Inline { file, line, col } => {
				format!("inline:{file}:{line}:{col}").into()
			}
		}
	}
}

impl core::fmt::Display for ClassName {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		self.as_selector().fmt(f)
	}
}


/// Classes assigned to an element entity, checked alongside the `class`
/// attribute by [`ElementView::contains_class`].
#[derive(Default, Component, Deref, DerefMut)]
pub struct Classes(HashSet<ClassName>);

impl Classes {
	pub fn new(classes: impl IntoIterator<Item: Into<SmolStr>>) -> Self {
		Self(classes.into_iter().map(|c| ClassName::string(c)).collect())
	}

	pub fn insert_class(&mut self, class: ClassName) -> &mut Self {
		self.0.insert(class);
		self
	}

	/// `true` if any contained class matches the given selector string.
	pub fn contains_selector(&self, class: &str) -> bool {
		self.0.iter().any(|c| c.as_selector() == class)
	}
}

impl FromIterator<ClassName> for Classes {
	fn from_iter<I: IntoIterator<Item = ClassName>>(iter: I) -> Self {
		Self(iter.into_iter().collect())
	}
}


/// Converts a `(Token, Value)` pair into a declaration for use with
/// [`inline_class`].
pub trait IntoDeclaration {
	fn into_declaration(self) -> (TokenKey, TokenValue);
}

impl<T, V> IntoDeclaration for (T, V)
where
	T: TypedToken + Into<Token>,
	T::Value: Typed + Serialize,
	V: Into<T::Value>,
{
	fn into_declaration(self) -> (TokenKey, TokenValue) {
		let (key, value) = self;
		let token: Token = key.into();
		let value = TypedValue::new(value.into())
			.expect("inline_class: failed to serialize value");
		(token.key().clone(), TokenValue::Value(value))
	}
}


/// Register a rule inline at the callsite, returning an [`OnSpawn`] effect that
/// adds a unique inline class to the entity and registers the rule (only once)
/// in the global [`RuleSet`].
/// This pattern is somewhat analagous to Component Scoped Styles as seen in frameworks
/// like Astro.
#[track_caller]
pub fn inline_class(
	declarations: impl IntoIterator<Item = (TokenKey, TokenValue)>,
) -> OnSpawn {
	let class = ClassName::new_inline();
	let selector = Selector::Class(class.as_selector());
	let declarations: Vec<(TokenKey, TokenValue)> =
		declarations.into_iter().collect();
	OnSpawn::new(move |entity| -> Result {
		let rule = Rule::new()
			.with_selector(selector)
			.with_extend(declarations);
		entity.world_scope(move |world| {
			world
				.get_resource_or_init::<RuleSet>()
				.try_insert_inline(rule);
		});
		if let Some(mut classes) = entity.get_mut::<Classes>() {
			classes.insert_class(class);
		} else {
			entity.insert(Classes::from_iter([class]));
		}
		Ok(())
	})
}

/// Declare a [`RuleSet`] rule inline on an element.
///
/// ```ignore
/// rsx!{<div {inline_class![
/// 	(ForegroundColor, Color::BLUE),
/// 	(BackgroundColor, Color::RED),
/// ]} />}
/// ```
#[macro_export]
macro_rules! inline_class {
	[$($decl:expr),* $(,)?] => {
		$crate::prelude::inline_class([
			$($crate::prelude::IntoDeclaration::into_declaration($decl)),*
		])
	};
}
