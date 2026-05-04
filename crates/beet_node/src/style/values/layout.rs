use crate::style::*;
use beet_core::prelude::*;


#[derive(Component)]
pub struct FlexBox {
	pub direction: Direction,
	pub layout_style: LayoutStyle,
	pub wrap: FlexWrap,
	pub align_items: AlignItems,
	pub align_content: AlignContent,
	pub justify_content: JustifyContent,
	pub row_gap: u32,
	pub column_gap: u32,
}

impl FlexBox {
	pub fn row() -> Self {
		Self {
			layout_style: LayoutStyle::default(),
			direction: Direction::Horizontal,
			wrap: FlexWrap::NoWrap,
			align_items: AlignItems::default(),
			align_content: AlignContent::default(),
			justify_content: JustifyContent::default(),
			row_gap: 0,
			column_gap: 0,
		}
	}
	pub fn col() -> Self {
		Self {
			layout_style: LayoutStyle::default(),
			direction: Direction::Vertical,
			wrap: FlexWrap::NoWrap,
			align_items: AlignItems::default(),
			align_content: AlignContent::default(),
			justify_content: JustifyContent::default(),
			row_gap: 0,
			column_gap: 0,
		}
	}
	pub fn wrap(mut self, wrap: FlexWrap) -> Self {
		self.wrap = wrap;
		self
	}
	pub fn align_items(mut self, align: AlignItems) -> Self {
		self.align_items = align;
		self
	}
	pub fn align_content(mut self, align: AlignContent) -> Self {
		self.align_content = align;
		self
	}
	pub fn justify_content(mut self, justify: JustifyContent) -> Self {
		self.justify_content = justify;
		self
	}
	pub fn row_gap(mut self, gap: u32) -> Self {
		self.row_gap = gap;
		self
	}
	pub fn column_gap(mut self, gap: u32) -> Self {
		self.column_gap = gap;
		self
	}
	pub fn gap(mut self, gap: u32) -> Self {
		self.row_gap = gap;
		self.column_gap = gap;
		self
	}
}



#[derive(Debug, Default, Clone, SetWith, Component)]
pub struct LayoutStyle {
	pub flex_order: i32,
	pub flex_grow: u32,
	pub align_self: AlignSelf,
	pub padding: Spacing,
	pub margin: Spacing,
	pub border: Spacing,
	pub text_align: TextAlign,
}

impl LayoutStyle {
	pub const DEFAULT: Self = Self {
		flex_order: 0,
		flex_grow: 0,
		align_self: AlignSelf::Auto,
		padding: Spacing::DEFAULT,
		margin: Spacing::DEFAULT,
		border: Spacing::DEFAULT,
		text_align: TextAlign::Left,
	};
}


#[derive(Debug, Default, Clone, Copy, PartialEq, Reflect)]
pub enum JustifyContent {
	#[default]
	Start,
	End,
	Center,
	SpaceBetween,
	SpaceEvenly,
	SpaceAround,
}

impl AsCssValue for JustifyContent {
	fn as_css_value(&self) -> Result<CssValue> {
		match self {
			Self::Start => "start",
			Self::End => "end",
			Self::Center => "center",
			Self::SpaceBetween => "space-between",
			Self::SpaceEvenly => "space-evenly",
			Self::SpaceAround => "space-around",
		}
		.to_string()
		.xmap(CssValue::expression)
		.xok()
	}
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Reflect)]
pub enum AlignItems {
	#[default]
	Start,
	End,
	Center,
	Stretch,
	Baseline,
}

impl AsCssValue for AlignItems {
	fn as_css_value(&self) -> Result<CssValue> {
		match self {
			Self::Start => "start",
			Self::End => "end",
			Self::Center => "center",
			Self::Stretch => "stretch",
			Self::Baseline => "baseline",
		}
		.xmap(CssValue::expression)
		.xok()
	}
}

#[derive(Debug, Default, Clone, PartialEq, Reflect)]
pub enum AlignSelf {
	#[default]
	Auto, // inherit from container's align_items
	Start,
	End,
	Center,
	Stretch,
	Baseline,
}

impl AsCssValue for AlignSelf {
	fn as_css_value(&self) -> Result<CssValue> {
		match self {
			Self::Auto => "auto",
			Self::Start => "start",
			Self::End => "end",
			Self::Center => "center",
			Self::Stretch => "stretch",
			Self::Baseline => "baseline",
		}
		.xmap(CssValue::expression)
		.xok()
	}
}

#[derive(Debug, Default, Clone, PartialEq, Reflect)]
pub enum FlexSize {
	#[default]
	Auto,
	Unit(Length),
	Grow(u16),
	Shrink(u16),
}

impl AsCssValue for FlexSize {
	fn as_css_value(&self) -> Result<CssValue> {
		match self {
			Self::Auto => CssValue::expression("auto"),
			Self::Unit(unit) => unit.as_css_value()?,
			Self::Grow(n) => n.as_css_value()?,
			Self::Shrink(n) => n.as_css_value()?,
		}
		.xok()
	}
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Reflect)]
pub enum Direction {
	#[default]
	Horizontal,
	Vertical,
	ViewportMin,
	ViewportMax,
}

impl AsCssValue for Direction {
	fn as_css_value(&self) -> Result<CssValue> {
		match self {
			Self::Horizontal => "horizontal",
			Self::Vertical => "vertical",
			Self::ViewportMin => "vmin",
			Self::ViewportMax => "vmax",
		}
		.xmap(CssValue::expression)
		.xok()
	}
}



#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum TextAlign {
	#[default]
	Left,
	Center,
	Right,
}

/// How to distribute lines along the cross axis when wrapping.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum AlignContent {
	#[default]
	Start,
	Center,
	End,
	SpaceBetween,
	SpaceAround,
	Stretch,
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum FlexWrap {
	#[default]
	NoWrap,
	Wrap,
}

/// Spacing around an element.
#[derive(Debug, Default, Clone, Copy, PartialEq, SetWith)]
pub struct Spacing {
	pub top: Length,
	pub right: Length,
	pub bottom: Length,
	pub left: Length,
}

impl Spacing {
	pub const DEFAULT: Self = Self {
		top: Length::DEFAULT,
		right: Length::DEFAULT,
		bottom: Length::DEFAULT,
		left: Length::DEFAULT,
	};

	pub fn all(length: Length) -> Self {
		Self {
			top: length,
			right: length,
			bottom: length,
			left: length,
		}
	}

	/// Create a URect where the min represents left and top,
	/// and the max represents right and bottom
	pub fn rem_urect(&self, viewport: Vec2) -> URect {
		let left = self.left.into_rem(viewport).round() as u32;
		let right = self.right.into_rem(viewport).round() as u32;
		let top = self.top.into_rem(viewport).round() as u32;
		let bottom = self.bottom.into_rem(viewport).round() as u32;

		URect {
			min: UVec2::new(left, top),
			max: UVec2::new(right, bottom),
		}
	}

	// pub fn horizontal(&self) -> Length { self.left + self.right }
	// pub fn vertical(&self) -> Length { self.top + self.bottom }
}
