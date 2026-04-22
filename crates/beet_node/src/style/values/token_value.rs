use crate::style::AlignItems;
use crate::style::AlignSelf;
use crate::style::Direction;
use crate::style::Elevation;
use crate::style::FlexSize;
use crate::style::FontWeight;
use crate::style::JustifyContent;
use crate::style::Shape;
use crate::style::Typeface;
use crate::style::Typography;
use crate::style::*;
use beet_core::prelude::*;

/// A concrete style value carried by a [`TokenStore`].
///
/// The `T` parameter allows custom extension values via `Custom(T)`.
/// The unit type `()` acts as the default "unset" custom variant.
#[derive(Debug, Clone, PartialEq, Reflect)]
pub enum TokenValue<T = ()> {
	Color(Color),
	Unit(Length),
	JustifyContent(JustifyContent),
	AlignItems(AlignItems),
	AlignSelf(AlignSelf),
	FlexSize(FlexSize),
	Direction(Direction),
	Typeface(Typeface),
	FontWeight(FontWeight),
	Typography(Typography),
	Elevation(Elevation),
	Shape(Shape),
	Motion(Motion),
	EaseFunction(EaseFunction),
	Duration(Duration),
	Scalar(f32),
	Custom(T),
}

/// The type tag delegates to the custom type `T`, representing the
/// "default" variant kind for this token value parameterisation.
impl<T: TypeTag> TypeTag for TokenValue<T> {
	const TYPE_TAG: SmolStr = T::TYPE_TAG;
}

impl<T: TypeTag> TokenValue<T> {
	/// Returns the CSS type-tag for the currently stored variant.
	pub fn type_tag(&self) -> SmolStr {
		match self {
			Self::Color(_) => Color::TYPE_TAG,
			Self::Unit(_) => Length::TYPE_TAG,
			Self::JustifyContent(_) => JustifyContent::TYPE_TAG,
			Self::AlignItems(_) => AlignItems::TYPE_TAG,
			Self::AlignSelf(_) => AlignSelf::TYPE_TAG,
			Self::FlexSize(_) => FlexSize::TYPE_TAG,
			Self::Direction(_) => Direction::TYPE_TAG,
			Self::Typeface(_) => Typeface::TYPE_TAG,
			Self::FontWeight(_) => FontWeight::TYPE_TAG,
			Self::Typography(_) => Typography::TYPE_TAG,
			Self::Elevation(_) => Elevation::TYPE_TAG,
			Self::Shape(_) => Shape::TYPE_TAG,
			Self::Motion(_) => Motion::TYPE_TAG,
			Self::EaseFunction(_) => EaseFunction::TYPE_TAG,
			Self::Duration(_) => Duration::TYPE_TAG,
			Self::Scalar(_) => f32::TYPE_TAG,
			Self::Custom(_) => T::TYPE_TAG,
		}
	}
}

impl<T: CssValue> CssValue for TokenValue<T> {
	fn to_css_value(&self) -> String {
		match self {
			Self::Color(v) => v.to_css_value(),
			Self::Unit(v) => v.to_css_value(),
			Self::JustifyContent(v) => v.to_css_value(),
			Self::AlignItems(v) => v.to_css_value(),
			Self::AlignSelf(v) => v.to_css_value(),
			Self::FlexSize(v) => v.to_css_value(),
			Self::Direction(v) => v.to_css_value(),
			Self::Typeface(v) => v.to_css_value(),
			Self::FontWeight(v) => v.to_css_value(),
			Self::Typography(v) => v.to_css_value(),
			Self::Motion(v) => v.to_css_value(),
			Self::Elevation(v) => v.to_css_value(),
			Self::Shape(v) => v.to_css_value(),
			Self::EaseFunction(v) => v.to_css_value(),
			Self::Duration(v) => v.to_css_value(),
			Self::Scalar(v) => v.to_css_value(),
			Self::Custom(v) => v.to_css_value(),
		}
	}
}

// Generic From impls: the Color/Unit/layout variants don't depend on T.

impl<T> From<Color> for TokenValue<T> {
	fn from(value: Color) -> Self { Self::Color(value) }
}

impl<T> From<Length> for TokenValue<T> {
	fn from(value: Length) -> Self { Self::Unit(value) }
}

impl<T> From<JustifyContent> for TokenValue<T> {
	fn from(value: JustifyContent) -> Self { Self::JustifyContent(value) }
}

impl<T> From<AlignItems> for TokenValue<T> {
	fn from(value: AlignItems) -> Self { Self::AlignItems(value) }
}

impl<T> From<AlignSelf> for TokenValue<T> {
	fn from(value: AlignSelf) -> Self { Self::AlignSelf(value) }
}

impl<T> From<FlexSize> for TokenValue<T> {
	fn from(value: FlexSize) -> Self { Self::FlexSize(value) }
}

impl<T> From<Direction> for TokenValue<T> {
	fn from(value: Direction) -> Self { Self::Direction(value) }
}

impl<T> From<Typeface> for TokenValue<T> {
	fn from(value: Typeface) -> Self { Self::Typeface(value) }
}

impl<T> From<FontWeight> for TokenValue<T> {
	fn from(value: FontWeight) -> Self { Self::FontWeight(value) }
}

impl<T> From<Motion> for TokenValue<T> {
	fn from(value: Motion) -> Self { Self::Motion(value) }
}

impl<T> From<Typography> for TokenValue<T> {
	fn from(value: Typography) -> Self { Self::Typography(value) }
}

impl<T> From<Elevation> for TokenValue<T> {
	fn from(value: Elevation) -> Self { Self::Elevation(value) }
}

impl<T> From<Shape> for TokenValue<T> {
	fn from(value: Shape) -> Self { Self::Shape(value) }
}

impl<T> From<EaseFunction> for TokenValue<T> {
	fn from(value: EaseFunction) -> Self { Self::EaseFunction(value) }
}

impl<T> From<Duration> for TokenValue<T> {
	fn from(value: Duration) -> Self { Self::Duration(value) }
}

impl<T> From<f32> for TokenValue<T> {
	fn from(value: f32) -> Self { Self::Scalar(value) }
}

impl TypeTag for Color {
	const TYPE_TAG: SmolStr = SmolStr::new_static("color");
}

impl CssValue for Color {
	fn to_css_value(&self) -> String {
		let this = self.to_srgba();
		format!(
			"rgba({}, {}, {}, {})",
			(this.red * 255.0).round() as u8,
			(this.green * 255.0).round() as u8,
			(this.blue * 255.0).round() as u8,
			this.alpha
		)
	}
}
