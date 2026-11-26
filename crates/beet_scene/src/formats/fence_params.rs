//! What a markdown fence says beyond the format it holds: [`FenceRender`],
//! what it becomes, and [`FenceParams`], how its format reads it.
use crate::prelude::*;
use beet_core::prelude::*;
use bevy::reflect::FromReflect;
use bevy::reflect::TypeRegistry;
use bevy::reflect::Typed;

/// What a markdown fence becomes, its `render` attribute: ```` ```html ````
/// is code and ```` ```html render ```` the document it holds. A format
/// declares its own default ([`DocParser::fence_render`]), so a diagram is
/// drawn unless it says `render=code`.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum FenceRender {
	/// A code sample, as written.
	#[default]
	Code,
	/// The document it holds, read in its format and standing where the
	/// fence does.
	Scene,
}

impl FenceRender {
	/// The `render` attribute's value: a bare `render` is `scene`.
	///
	/// ## Errors
	/// Errors on any other value.
	pub fn from_literal(literal: &DataLiteral) -> Result<Self> {
		let word = match literal {
			DataLiteral::Scalar(Value::Bool(true)) => return Self::Scene.xok(),
			DataLiteral::Scalar(Value::Str(word)) => word.as_str(),
			DataLiteral::Enum(named) if named.fields == NamedFields::Unit => {
				named.name.as_str()
			}
			_ => "",
		};
		match word {
			"code" => Self::Code.xok(),
			"scene" => Self::Scene.xok(),
			_ => bevybail!(
				"a fence's `render` is `code` or `scene`, or bare for `scene`"
			),
		}
	}
}

/// What a markdown fence says beyond the format it holds and what it
/// becomes, each param written as a BSX attribute, ie `form=text` of
/// ```` ```mermaid form=text ````. A parser [configured](DocParser::configure)
/// with them reads them into a params type of its own
/// ([`read`](Self::read)), as a tag's attributes are read into its component.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct FenceParams(Vec<(SmolStr, DataLiteral)>);

impl FenceParams {
	/// Params by name, in the order written.
	pub fn new(params: Vec<(SmolStr, DataLiteral)>) -> Self { Self(params) }

	/// Whether the fence says nothing beyond its format.
	pub fn is_empty(&self) -> bool { self.0.is_empty() }

	/// Refuse any param, what a format taking none answers.
	pub fn assert_empty(&self) -> Result {
		match self.0.as_slice() {
			[] => Ok(()),
			params => bevybail!(
				"this format takes no fence params, found {}",
				params
					.iter()
					.map(|(key, _)| format!("`{key}`"))
					.collect::<Vec<_>>()
					.join(", ")
			),
		}
	}

	/// These params as `T`, each a field of it, every field unsaid its
	/// default.
	///
	/// ## Errors
	/// Errors on a param naming no field of `T`, or a value that does not fit
	/// its field.
	pub fn read<T: Default + FromReflect + Typed>(&self) -> Result<T> {
		let mut value = T::default();
		if self.is_empty() {
			return value.xok();
		}
		// a params type is self-describing, so nested lookups never reach the
		// (empty) registry, and a fence names no entity to reference
		let patch = LiteralReflect::resolve(
			&DataLiteral::Struct(self.0.clone()),
			Some(T::type_info()),
			&TypeRegistry::empty(),
			&mut |_| Entity::PLACEHOLDER,
		)?;
		value.try_apply(patch.as_partial_reflect())?;
		value.xok()
	}
}
