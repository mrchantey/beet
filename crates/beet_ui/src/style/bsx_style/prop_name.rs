//! Markup-friendly names for the [`common_props`](crate::style::common_props)
//! property tokens and the Material colour-role tokens.
//!
//! The typed Rust API names a declaration by its token type
//! (`common_props::DisplayProp`) and its value by the Rust enum form
//! (`Display::Flex`). Markup has no types: a `<Rule>` keys each declaration by a
//! kebab-case property name (`display`, `column-gap`) and parses the value
//! through the BSX value grammar. These maps bridge the two:
//!
//! - [`prop_name_map`]: a kebab property name -> a [`PropResolver`] that parses a
//!   [`DataLiteral`] into the same [`TokenValue`] the typed `with_value` produces;
//! - [`color_role_map`]: a Material role name (`Primary`) -> its token, for the
//!   `@token:Role` token-to-token binding form.
use crate::prelude::*;
use crate::style::common_props::*;
use crate::style::material::colors;
use beet_core::prelude::*;
use bevy::reflect::FromReflect;
use bevy::reflect::TypeRegistry;
use bevy::reflect::Typed;

/// Resolves a `<Rule>` declaration value (a parsed [`DataLiteral`] in BSX
/// enum-form, eg `Flex` or `Rem(1.0)`) into a [`TokenValue`] for a known
/// property token. The token and parser are paired so the value coerces against
/// the token's value type identically to the typed `Rule::with_value` API.
#[derive(Clone)]
pub struct PropResolver {
	/// The property token this name addresses, eg [`DisplayProp`].
	pub token: Token,
	/// Parse a literal value into the token's typed [`TokenValue`].
	parse: fn(&DataLiteral) -> Result<TokenValue>,
}

impl PropResolver {
	/// Parse `literal` into this property's [`TokenValue`].
	pub fn parse(&self, literal: &DataLiteral) -> Result<TokenValue> {
		(self.parse)(literal)
	}
}

/// A [`PropResolver`] for property token `T`, whose value parses against
/// `T::Value`'s reflected type (no type registry needed, the value types are
/// self-describing through [`Typed`]).
fn prop<T>() -> PropResolver
where
	T: 'static + TypedToken + Into<Token> + Default,
	T::Value: FromReflect + Typed + Serialize,
{
	PropResolver {
		token: T::default().into(),
		parse: parse_value::<T::Value>,
	}
}

/// Parse a literal into `V` the same way the typed API serializes it: resolve
/// the literal against `V`'s reflected type, then `V::from_reflect` and
/// [`TokenValue::value`] (ie `Value::from_serde(V)`), so a markup `Rem(1.0)`
/// round-trips identically to `with_value(.., Length::Rem(1.0))`.
///
/// A struct value (eg `Spacing`) gives every field, the markup twin of the typed
/// `Spacing { bottom: .., ..Spacing::DEFAULT }`, since `from_reflect` rejects a
/// struct literal missing any field.
fn parse_value<V>(literal: &DataLiteral) -> Result<TokenValue>
where
	V: FromReflect + Typed + Serialize,
{
	// the value types are self-describing, so nested-type lookups never hit the
	// (empty) registry; `$name` entity refs are meaningless in a rule value.
	let registry = TypeRegistry::empty();
	let reflected = literal_to_reflect(
		literal,
		Some(V::type_info()),
		&registry,
		&mut |_| Entity::PLACEHOLDER,
	)?;
	let value = V::from_reflect(reflected.as_ref()).ok_or_else(|| {
		bevyhow!("value `{literal:?}` is not a valid `{}`", V::type_path())
	})?;
	TokenValue::value(value)
}

/// Maps a kebab-case CSS property name to the [`PropResolver`] for its
/// [`common_props`](crate::style::common_props) token. The keys mirror the
/// property strings the `css_property!` declarations emit, so a `<Rule>`
/// attribute reads like the CSS it produces.
pub fn prop_name_map() -> HashMap<SmolStr, PropResolver> {
	[
		("color", prop::<ForegroundColor>()),
		("background-color", prop::<BackgroundColor>()),
		("display", prop::<DisplayProp>()),
		("flex-direction", prop::<FlexDirectionProp>()),
		("flex-wrap", prop::<FlexWrapProp>()),
		("flex-grow", prop::<FlexGrowProp>()),
		("align-items", prop::<AlignItemsProp>()),
		("align-content", prop::<AlignContentProp>()),
		("align-self", prop::<AlignSelfProp>()),
		("justify-content", prop::<JustifyContentProp>()),
		("gap", prop::<GapProp>()),
		("column-gap", prop::<ColumnGapProp>()),
		("row-gap", prop::<RowGapProp>()),
		("grid-template-columns", prop::<GridTemplateColumnsProp>()),
		("grid-auto-rows", prop::<GridAutoRowsProp>()),
		("width", prop::<Width>()),
		("min-width", prop::<MinWidth>()),
		("max-width", prop::<MaxWidth>()),
		("height", prop::<Height>()),
		("min-height", prop::<MinHeight>()),
		("max-height", prop::<MaxHeight>()),
		("padding", prop::<Padding>()),
		("margin", prop::<MarginProp>()),
		("border-radius", prop::<ShapeProp>()),
		("box-shadow", prop::<ElevationProp>()),
		("border-color", prop::<BorderColorProp>()),
		("opacity", prop::<OpacityProp>()),
		("overflow-x", prop::<OverflowXProp>()),
		("overflow-y", prop::<OverflowYProp>()),
		("position", prop::<PositionProp>()),
		("top", prop::<InsetTop>()),
		("right", prop::<InsetRight>()),
		("bottom", prop::<InsetBottom>()),
		("left", prop::<InsetLeft>()),
		("z-index", prop::<ZIndexProp>()),
		("cursor", prop::<CursorProp>()),
		("transform", prop::<TransformProp>()),
		("font-size", prop::<FontSize>()),
		("line-height", prop::<LineHeight>()),
		("letter-spacing", prop::<Tracking>()),
		("text-align", prop::<TextAlignProp>()),
		("white-space", prop::<WhiteSpaceProp>()),
		("list-style-type", prop::<ListStyleProp>()),
	]
	.into_iter()
	.map(|(name, resolver)| (SmolStr::new_static(name), resolver))
	.collect()
}

/// Maps a Material colour-role name (`Primary`, `OnSurface`) to its token, the
/// target of a `<Rule>` `@token:Role` binding. The names match the
/// [`material::colors`](crate::style::material::colors) token idents.
pub fn color_role_map() -> HashMap<SmolStr, Token> {
	use colors::*;
	[
		("Primary", Primary.into()),
		("OnPrimary", OnPrimary.into()),
		("PrimaryContainer", PrimaryContainer.into()),
		("OnPrimaryContainer", OnPrimaryContainer.into()),
		("InversePrimary", InversePrimary.into()),
		("Secondary", Secondary.into()),
		("OnSecondary", OnSecondary.into()),
		("SecondaryContainer", SecondaryContainer.into()),
		("OnSecondaryContainer", OnSecondaryContainer.into()),
		("Tertiary", Tertiary.into()),
		("OnTertiary", OnTertiary.into()),
		("TertiaryContainer", TertiaryContainer.into()),
		("OnTertiaryContainer", OnTertiaryContainer.into()),
		("Error", Error.into()),
		("OnError", OnError.into()),
		("ErrorContainer", ErrorContainer.into()),
		("OnErrorContainer", OnErrorContainer.into()),
		("Surface", Surface.into()),
		("SurfaceDim", SurfaceDim.into()),
		("SurfaceBright", SurfaceBright.into()),
		("SurfaceContainerLowest", SurfaceContainerLowest.into()),
		("SurfaceContainerLow", SurfaceContainerLow.into()),
		("SurfaceContainer", SurfaceContainer.into()),
		("SurfaceContainerHigh", SurfaceContainerHigh.into()),
		("SurfaceContainerHighest", SurfaceContainerHighest.into()),
		("OnSurface", OnSurface.into()),
		("OnSurfaceVariant", OnSurfaceVariant.into()),
		("SurfaceVariant", SurfaceVariant.into()),
		("InverseSurface", InverseSurface.into()),
		("InverseOnSurface", InverseOnSurface.into()),
		("Outline", Outline.into()),
		("OutlineVariant", OutlineVariant.into()),
		("Background", Background.into()),
		("OnBackground", OnBackground.into()),
		("Shadow", Shadow.into()),
		("Scrim", Scrim.into()),
	]
	.into_iter()
	.map(|(name, token)| (SmolStr::new_static(name), token))
	.collect()
}
