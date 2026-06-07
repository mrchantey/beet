use crate::style::*;
use beet_core::prelude::*;

/// Display algorithm for a node.
#[derive(Debug, Default, Clone, Copy, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Display {
	/// Standard block flow layout.
	#[default]
	Block,
	/// Inline flow layout.
	Inline,
	/// Flexbox layout.
	Flex,
	/// Removed from layout entirely (hidden), mapping to CSS `display: none`.
	None,
}

impl AsCssValue for Display {
	fn as_css_value(&self) -> Result<CssValue> {
		match self {
			Self::Block => "block",
			Self::Inline => "inline",
			Self::Flex => "flex",
			Self::None => "none",
		}
		.xmap(CssValue::expression)
		.xok()
	}
}

/// Mouse cursor shown over a node, mapping to CSS `cursor`. Web-only; the
/// terminal has no pointer cursor so the charcell cascade ignores it.
#[derive(Debug, Default, Clone, Copy, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Cursor {
	/// Browser default for the element.
	#[default]
	Auto,
	/// The hand pointer, signalling an interactive control.
	Pointer,
}

impl AsCssValue for Cursor {
	fn as_css_value(&self) -> Result<CssValue> {
		match self {
			Self::Auto => "auto",
			Self::Pointer => "pointer",
		}
		.xmap(CssValue::expression)
		.xok()
	}
}

/// A CSS `transform`. Only the rotation beet needs today is modelled; the
/// terminal has no transform, so the charcell cascade ignores the property.
#[derive(Debug, Default, Clone, Copy, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Transform {
	/// No transform applied.
	#[default]
	None,
	/// Clockwise rotation in degrees about the box centre.
	Rotate(f32),
}

impl AsCssValue for Transform {
	fn as_css_value(&self) -> Result<CssValue> {
		match self {
			Self::None => "none".to_string(),
			Self::Rotate(degrees) => format!("rotate({degrees}deg)"),
		}
		.xmap(CssValue::expression)
		.xok()
	}
}

/// Fragmentation break forced after a box, mapping to CSS `break-after`.
///
/// Only meaningful for paginated media (print); the `@media print` rule that
/// uses it is ignored by the non-web cascade.
#[derive(Debug, Default, Clone, Copy, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum BreakAfter {
	/// No forced break.
	#[default]
	Auto,
	/// Always force a page break after the box.
	Page,
}

impl AsCssValue for BreakAfter {
	fn as_css_value(&self) -> Result<CssValue> {
		match self {
			Self::Auto => "auto",
			Self::Page => "page",
		}
		.xmap(CssValue::expression)
		.xok()
	}
}

/// How whitespace and line breaks inside a node are handled during text flow.
#[derive(Debug, Default, Clone, Copy, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum WhiteSpace {
	/// Collapse runs of whitespace and wrap lines at the content width.
	#[default]
	Normal,
	/// Preserve whitespace and newlines verbatim, breaking only on `\n`.
	Pre,
}

impl AsCssValue for WhiteSpace {
	fn as_css_value(&self) -> Result<CssValue> {
		match self {
			Self::Normal => "normal",
			Self::Pre => "pre",
		}
		.xmap(CssValue::expression)
		.xok()
	}
}

/// Marker style for list items, mapping to CSS `list-style-type`.
///
/// Inherited (like CSS), so setting `None` on an ancestor (eg a `<nav>`) strips
/// markers from every descendant list item. The charcell decorator reads the
/// resolved value to decide whether a `<li>` gets a bullet/number.
#[derive(Debug, Default, Clone, Copy, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ListStyle {
	/// Marker chosen from the list kind (`<ul>` bullet, `<ol>` number).
	#[default]
	Auto,
	/// No marker, mapping to CSS `list-style-type: none`.
	None,
}

impl AsCssValue for ListStyle {
	fn as_css_value(&self) -> Result<CssValue> {
		match self {
			// `Auto` keeps the browser's per-tag default (disc/decimal); only the
			// `None` opt-out needs serializing.
			Self::Auto => "revert",
			Self::None => "none",
		}
		.xmap(CssValue::expression)
		.xok()
	}
}

pub static LAYOUT_STYLE_DEFAULT: LayoutStyle = LayoutStyle::DEFAULT;

/// Layout properties for a node.
#[derive(Debug, Default, Clone, PartialEq, SetWith, Component)]
pub struct LayoutStyle {
	pub display: Display,
	pub white_space: WhiteSpace,
	pub flex_box: FlexBox,
	pub flex_order: i32,
	pub flex_grow: u32,
	pub align_self: AlignSelf,
}

impl LayoutStyle {
	pub const DEFAULT: Self = Self {
		display: Display::Block,
		white_space: WhiteSpace::Normal,
		flex_box: FlexBox::row(),
		flex_order: 0,
		flex_grow: 0,
		align_self: AlignSelf::Auto,
	};

	/// Create a flex-row layout style.
	pub fn flex_row() -> Self {
		Self {
			display: Display::Flex,
			flex_box: FlexBox::row(),
			..default()
		}
	}

	/// Create a flex-column layout style.
	pub fn flex_col() -> Self {
		Self {
			display: Display::Flex,
			flex_box: FlexBox::col(),
			..default()
		}
	}

	pub fn justify_content(mut self, justify: JustifyContent) -> Self {
		self.flex_box.justify_content = justify;
		self
	}

	pub fn align_items(mut self, align: AlignItems) -> Self {
		self.flex_box.align_items = align;
		self
	}

	pub fn align_content(mut self, align: AlignContent) -> Self {
		self.flex_box.align_content = align;
		self
	}

	pub fn wrap(mut self, wrap: FlexWrap) -> Self {
		self.flex_box.wrap = wrap;
		self
	}

	pub fn row_gap(mut self, gap: Length) -> Self {
		self.flex_box.row_gap = gap;
		self
	}

	pub fn column_gap(mut self, gap: Length) -> Self {
		self.flex_box.column_gap = gap;
		self
	}

	pub fn gap(mut self, gap: Length) -> Self {
		self.flex_box.row_gap = gap;
		self.flex_box.column_gap = gap;
		self
	}
}

/// Flexbox configuration for a node.
///
/// Gaps are stored as [`Length`] (resolution-independent), not pre-rounded
/// cells: a pixel native renderer wants pixels and a viewport-relative gap needs
/// the real viewport, so each target converts at layout time. The charcell
/// engine uses [`FlexBox::row_gap_cells`]/[`FlexBox::column_gap_cells`].
#[derive(Debug, Clone, PartialEq)]
pub struct FlexBox {
	pub direction: Direction,
	pub wrap: FlexWrap,
	pub align_items: AlignItems,
	pub align_content: AlignContent,
	pub justify_content: JustifyContent,
	pub row_gap: Length,
	pub column_gap: Length,
}

impl Default for FlexBox {
	fn default() -> Self { Self::row() }
}

impl FlexBox {
	pub const fn row() -> Self {
		Self {
			direction: Direction::Horizontal,
			wrap: FlexWrap::NoWrap,
			align_items: AlignItems::Start,
			align_content: AlignContent::Start,
			justify_content: JustifyContent::Start,
			row_gap: Length::DEFAULT,
			column_gap: Length::DEFAULT,
		}
	}

	pub const fn col() -> Self {
		Self {
			direction: Direction::Vertical,
			wrap: FlexWrap::NoWrap,
			align_items: AlignItems::Start,
			align_content: AlignContent::Start,
			justify_content: JustifyContent::Start,
			row_gap: Length::DEFAULT,
			column_gap: Length::DEFAULT,
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

	pub fn row_gap(mut self, gap: Length) -> Self {
		self.row_gap = gap;
		self
	}

	pub fn column_gap(mut self, gap: Length) -> Self {
		self.column_gap = gap;
		self
	}

	pub fn gap(mut self, gap: Length) -> Self {
		self.row_gap = gap;
		self.column_gap = gap;
		self
	}

	/// The row gap rounded to whole terminal cells, resolving any
	/// viewport-relative length against `viewport` (in cells).
	pub fn row_gap_cells(&self, viewport: UVec2) -> u32 {
		gap_cells(self.row_gap, viewport)
	}

	/// The column gap rounded to whole terminal cells, resolving any
	/// viewport-relative length against `viewport` (in cells).
	pub fn column_gap_cells(&self, viewport: UVec2) -> u32 {
		gap_cells(self.column_gap, viewport)
	}
}

/// Round a gap [`Length`] to whole terminal cells (1rem ≈ 1 cell), matching how
/// [`Spacing`] insets convert in the charcell box model.
fn gap_cells(gap: Length, viewport: UVec2) -> u32 {
	gap.into_rem(viewport.as_vec2()).round().max(0.) as u32
}


#[derive(Debug, Default, Clone, Copy, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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
		.xmap(CssValue::expression)
		.xok()
	}
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Direction {
	#[default]
	Horizontal,
	Vertical,
	ViewportMin,
	ViewportMax,
}

impl AsCssValue for Direction {
	fn as_css_value(&self) -> Result<CssValue> {
		// `flex-direction` is the only CSS consumer, so map to its axis keywords.
		// The charcell layout engine reads the [`Direction`] enum directly, not
		// this string, so the viewport-relative variants (charcell-only) collapse
		// to their dominant axis here.
		match self {
			Self::Horizontal | Self::ViewportMin => "row",
			Self::Vertical | Self::ViewportMax => "column",
		}
		.xmap(CssValue::expression)
		.xok()
	}
}


/// How to distribute lines along the cross axis when wrapping.
#[derive(Debug, Default, Clone, Copy, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum AlignContent {
	#[default]
	Start,
	Center,
	End,
	SpaceBetween,
	SpaceAround,
	Stretch,
}
impl AsCssValue for AlignContent {
	fn as_css_value(&self) -> Result<CssValue> {
		match self {
			Self::Start => "start",
			Self::Center => "center",
			Self::End => "end",
			Self::SpaceBetween => "space-between",
			Self::SpaceAround => "space-around",
			Self::Stretch => "stretch",
		}
		.xmap(CssValue::expression)
		.xok()
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Default, Reflect)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum FlexWrap {
	#[default]
	NoWrap,
	Wrap,
}
impl AsCssValue for FlexWrap {
	fn as_css_value(&self) -> Result<CssValue> {
		match self {
			Self::NoWrap => "nowrap",
			Self::Wrap => "wrap",
		}
		.xmap(CssValue::expression)
		.xok()
	}
}


/// Spacing around an element.
#[derive(Debug, Default, Clone, Copy, PartialEq, SetWith, Reflect)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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

impl AsCssValue for Spacing {
	fn as_css_value(&self) -> Result<CssValue> {
		format!("{} {} {} {}", self.top, self.right, self.bottom, self.left)
			.xmap(CssValue::expression)
			.xok()
	}
}
