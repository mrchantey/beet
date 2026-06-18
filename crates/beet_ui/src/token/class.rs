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
	/// A `const`-constructible class name from a static string, for declaring
	/// the shared class-name vocabulary as constants.
	pub const fn new_static(name: &'static str) -> Self {
		Self::String(SmolStr::new_static(name))
	}

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

	/// Create an inline class from a BSX source [`FileSpan`], the markup twin of
	/// [`Self::new_inline`]: the `bx:style` directive's span maps one-to-one onto
	/// the `panic::Location` a Rust `inline_class!` derives from.
	pub fn from_span(span: &FileSpan) -> Self {
		Self::Inline {
			file: span.path().to_string().into(),
			line: span.start_line(),
			col: span.start_col(),
		}
	}

	/// Create an inline class keyed on a `bx:style` directive's declaration
	/// `source`, the content-addressed twin of [`Self::from_span`].
	///
	/// A BSX source span is fragment-relative (markdown feeds each HTML block to
	/// the parser as its own string starting at offset 0), so two `bx:style`
	/// directives on different file lines can share a span and collide into one
	/// class, clobbering each other's rule. Keying on the declaration content
	/// instead is collision-free and correctly dedups identical declarations: a
	/// one-off rule is a pure function of its `prop=value` pairs.
	pub fn from_inline_source(source: &str) -> Self {
		let hash = source.bytes().fold(0u64, |acc, byte| {
			acc.wrapping_mul(31).wrapping_add(byte as u64)
		});
		Self::Inline {
			file: SmolStr::new_static("style"),
			line: (hash >> 32) as u32,
			col: hash as u32,
		}
	}

	/// The string used when matching against [`Selector::Class`],
	/// the class name does not have a `.` prefix.
	pub fn as_selector(&self) -> SmolStr {
		match self {
			Self::String(s) => s.clone(),
			// sanitize the callsite into a valid CSS identifier: the raw
			// `file:line:col` carries `/`, `.`, `:` which a browser would parse as
			// pseudo-class/combinator tokens, so the web rule would never match.
			Self::Inline { file, line, col } => {
				let file: String = file
					.chars()
					.map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
					.collect();
				format!("inline-{file}-{line}-{col}").into()
			}
		}
	}
}

impl core::fmt::Display for ClassName {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		self.as_selector().fmt(f)
	}
}

/// Anything that converts into a [`SmolStr`] (`&str`, `String`, `SmolStr`, …)
/// is a class name. [`ClassName`] itself converts via the reflexive blanket, so
/// both flow through `impl Into<ClassName>` APIs like [`Classes::new`].
impl<S: Into<SmolStr>> From<S> for ClassName {
	fn from(value: S) -> Self { Self::String(value.into()) }
}

/// Classes assigned to an element entity, checked alongside the `class`
/// attribute by [`ElementView::contains_class`].
#[derive(Default, Clone, Component, Deref, DerefMut)]
pub struct Classes(HashSet<ClassName>);

impl Classes {
	pub fn new(classes: impl IntoIterator<Item: Into<ClassName>>) -> Self {
		Self(classes.into_iter().map(Into::into).collect())
	}

	pub fn insert_class(&mut self, class: ClassName) -> &mut Self {
		self.0.insert(class);
		self
	}

	/// `true` if any contained class matches the given selector string.
	pub fn contains_selector(&self, class: &str) -> bool {
		self.0.iter().any(|c| c.as_selector() == class)
	}

	/// `true` if this set contains the given [`ClassName`]. Prefer this over
	/// [`Self::contains_selector`] when asserting against the shared class-name
	/// constants, keeping widget output and style rules in lockstep.
	pub fn contains_name(&self, class: &ClassName) -> bool {
		self.0.contains(class)
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

#[cfg(feature = "serde")]
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
///
/// [`OnSpawn`] is a [`BundleEffect`], so it works as a block attribute in both
/// the bundle `rsx!` and the scene `rsx!` lowerings (scenes lift it via
/// [`IntoScene`](crate::prelude::IntoScene)). This pattern is somewhat analagous
/// to Component Scoped Styles as seen in frameworks like Astro.
#[track_caller]
pub fn inline_class(
	declarations: impl IntoIterator<Item = (TokenKey, TokenValue)>,
) -> OnSpawn {
	let class = ClassName::new_inline();
	let rule = Rule::new()
		.with_selector(Selector::Class(class.as_selector()))
		.with_extend(declarations);
	OnSpawn::new(move |entity| {
		register_inline_rule(entity, class, rule.clone())
	})
}

/// Register a one-off `rule` keyed on `class` into the global [`RuleSet`] (only
/// once) and add `class` to the element's [`Classes`].
///
/// The shared runtime of the `inline_class!` macro and the `bx:style` markup
/// directive: the macro mints `class` from `panic::Location`, `bx:style` mints it
/// from the BSX source span, but both register the rule and attach the class
/// identically. The caller seeds `rule` with `Selector::Class(class.as_selector())`.
pub fn register_inline_rule(
	entity: &mut EntityWorldMut,
	class: ClassName,
	rule: Rule,
) -> Result {
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
