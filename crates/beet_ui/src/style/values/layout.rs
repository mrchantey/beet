use crate::style::*;
use beet_core::prelude::*;

/// Display algorithm for a node.
#[derive(Debug, Default, Clone, Copy, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Display {
	/// Standard block flow layout.
	#[default]
	Block,
	/// A block-level box that also generates a list marker, mapping to CSS
	/// `display: list-item`. The charcell engine lays it out exactly like
	/// [`Block`](Self::Block) (its marker comes from the decorator, not the
	/// display mode); the web needs `list-item` rather than `block` for a `<li>`
	/// to keep its bullet/number.
	ListItem,
	/// Inline flow layout.
	Inline,
	/// Flexbox layout.
	Flex,
	/// Grid layout: children flow row-major into equal-width column tracks
	/// (see [`GridTracks`]).
	Grid,
	/// Table wrapper (`<table>`): lays its rows out as a column-aligned grid. The
	/// charcell engine drives table layout off this and [`TableCell`](Self::TableCell)
	/// alone — rows and row groups (`<tr>`/`<thead>`/…) are found structurally, so
	/// they need no display of their own.
	Table,
	/// A table cell (`<td>`/`<th>`), mapping to CSS `display: table-cell`.
	TableCell,
	/// Removed from layout entirely (hidden), mapping to CSS `display: none`.
	None,
}

impl AsCssValue for Display {
	fn as_css_value(&self) -> Result<CssValue> {
		match self {
			Self::Block => "block",
			Self::ListItem => "list-item",
			Self::Inline => "inline",
			Self::Flex => "flex",
			Self::Grid => "grid",
			Self::Table => "table",
			Self::TableCell => "table-cell",
			Self::None => "none",
		}
		.xmap(CssValue::expression)
		.xok()
	}
}

/// The column count of a grid container, mapping to CSS
/// `grid-template-columns: repeat(n, minmax(0, 1fr))`. Defaults to the
/// familiar 12-column grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct GridColumns(pub u32);

impl Default for GridColumns {
	fn default() -> Self { Self(12) }
}

impl AsCssValue for GridColumns {
	fn as_css_value(&self) -> Result<CssValue> {
		format!("repeat({}, minmax(0, 1fr))", self.0)
			.xmap(CssValue::expression)
			.xok()
	}
}

/// Row track height of a grid container, mapping to CSS `grid-auto-rows`.
///
/// `Square` (default) sizes each track square: half its column width on the
/// terminal, where a cell is about twice as tall as wide. The web has no
/// square keyword, so it serializes to `auto` (pair with an `aspect-ratio`
/// rule there if squareness matters).
#[derive(Debug, Default, Clone, Copy, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum GridRows {
	/// Square tracks: row height = column width / 2 in cells.
	#[default]
	Square,
	/// An explicit track height.
	Length(Length),
}

impl GridRows {
	/// The track height in cells for a `column_width`-cell track.
	pub fn cells(&self, column_width: u32, viewport: UVec2) -> u32 {
		match self {
			Self::Square => (column_width / 2).max(1),
			Self::Length(length) => {
				length.into_rem(viewport.as_vec2()).round().max(1.) as u32
			}
		}
	}
}

impl AsCssValue for GridRows {
	fn as_css_value(&self) -> Result<CssValue> {
		match self {
			Self::Square => CssValue::expression("auto").xok(),
			Self::Length(length) => length.as_css_value(),
		}
	}
}

/// Grid track configuration for a `display: grid` container: the column count
/// ("width", defaulting to 12) and row track height ("height", defaulting to
/// square). Gaps come from the shared [`FlexBox`] gap values, like CSS `gap`.
#[derive(Debug, Clone, Copy, PartialEq, Reflect)]
pub struct GridTracks {
	pub columns: GridColumns,
	pub rows: GridRows,
}

impl Default for GridTracks {
	fn default() -> Self { Self::DEFAULT }
}

impl GridTracks {
	pub const DEFAULT: Self = Self {
		columns: GridColumns(12),
		rows: GridRows::Square,
	};
}

/// How content overflowing a node's box is handled along one axis, mapping to
/// CSS `overflow-x`/`overflow-y`.
///
/// `Visible` (default) lets content paint outside the box, the current behavior.
/// The other three clip descendants to the padding box; `Scroll` always reserves
/// a scrollbar gutter, `Auto` only when content overflows (Task 04).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Reflect)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Overflow {
	/// Content is not clipped and may paint outside the box.
	#[default]
	Visible,
	/// Content is clipped to the padding box, no scrolling affordance.
	Hidden,
	/// Content is clipped and always shows a scrollbar.
	Scroll,
	/// Content is clipped and shows a scrollbar only when it overflows.
	Auto,
}

impl Overflow {
	/// Whether this axis clips its overflow (anything but `Visible`).
	pub fn is_clipped(&self) -> bool { !matches!(self, Self::Visible) }

	/// Whether this axis is scrollable (`Scroll` or `Auto`).
	pub fn is_scroll(&self) -> bool {
		matches!(self, Self::Scroll | Self::Auto)
	}
}

impl AsCssValue for Overflow {
	fn as_css_value(&self) -> Result<CssValue> {
		match self {
			Self::Visible => "visible",
			Self::Hidden => "hidden",
			Self::Scroll => "scroll",
			Self::Auto => "auto",
		}
		.xmap(CssValue::expression)
		.xok()
	}
}

/// CSS `position`: how a box is placed relative to normal flow.
///
/// `Static` (default) is normal flow. `Relative` lays out in flow then offsets
/// by the insets. `Absolute`/`Fixed` are removed from flow and placed against a
/// containing block (the nearest positioned ancestor / the viewport). `Sticky`
/// lays out in flow then clamps within its scroll container.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Reflect)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Position {
	/// Normal flow, insets ignored.
	#[default]
	Static,
	/// Normal flow, then offset by the insets; the flow slot is preserved.
	Relative,
	/// Out of flow, placed against the nearest positioned ancestor's padding box.
	Absolute,
	/// Out of flow, placed against the viewport.
	Fixed,
	/// Normal flow, then clamped within the nearest scroll container's scrollport.
	Sticky,
}

impl Position {
	/// Whether the box is removed from its parent's normal flow.
	pub fn is_out_of_flow(&self) -> bool {
		matches!(self, Self::Absolute | Self::Fixed)
	}

	/// Whether the box establishes a containing block for absolute descendants.
	pub fn is_positioned(&self) -> bool { !matches!(self, Self::Static) }
}

impl AsCssValue for Position {
	fn as_css_value(&self) -> Result<CssValue> {
		match self {
			Self::Static => "static",
			Self::Relative => "relative",
			Self::Absolute => "absolute",
			Self::Fixed => "fixed",
			Self::Sticky => "sticky",
		}
		.xmap(CssValue::expression)
		.xok()
	}
}

/// Resolved positioning for a node: its [`Position`] and the four inset
/// (`top`/`right`/`bottom`/`left`) lengths, each `None` for CSS `auto`.
///
/// A dedicated component (rather than fields on [`LayoutStyle`]) keeps the common
/// layout style small; only positioned elements carry one. The charcell layout
/// pass reads it to place the box.
#[derive(Debug, Default, Clone, Copy, PartialEq, SetWith, Component)]
pub struct PositionStyle {
	pub position: Position,
	/// `[top, right, bottom, left]`, `None` for `auto`.
	pub inset: [Option<Length>; 4],
	/// CSS `z-index`: stacking order within the parent stacking context. `None`
	/// is `auto` (does not form a stacking context on its own).
	pub z_index: Option<i32>,
}

impl PositionStyle {
	pub const TOP: usize = 0;
	pub const RIGHT: usize = 1;
	pub const BOTTOM: usize = 2;
	pub const LEFT: usize = 3;

	pub fn top(&self) -> Option<Length> { self.inset[Self::TOP] }
	pub fn right(&self) -> Option<Length> { self.inset[Self::RIGHT] }
	pub fn bottom(&self) -> Option<Length> { self.inset[Self::BOTTOM] }
	pub fn left(&self) -> Option<Length> { self.inset[Self::LEFT] }

	/// Whether this style places the box anywhere but normal static flow.
	pub fn is_positioned(&self) -> bool { self.position.is_positioned() }
}

/// CSS `scrollbar-width`: the thickness of a scroll container's scrollbar.
///
/// `Auto` is the normal bar, `Thin` a lighter one (same 1-cell gutter, lighter
/// glyph in the terminal), `None` removes the bar and its reserved gutter.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Reflect)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ScrollbarWidth {
	/// The default scrollbar.
	#[default]
	Auto,
	/// A thinner scrollbar (lighter glyph), same reserved gutter.
	Thin,
	/// No scrollbar and no reserved gutter; content uses the full width.
	None,
}

impl ScrollbarWidth {
	/// Whether a gutter/bar is reserved at all (`None` reserves nothing).
	pub fn reserves_gutter(&self) -> bool { !matches!(self, Self::None) }
}

impl AsCssValue for ScrollbarWidth {
	fn as_css_value(&self) -> Result<CssValue> {
		match self {
			Self::Auto => "auto",
			Self::Thin => "thin",
			Self::None => "none",
		}
		.xmap(CssValue::expression)
		.xok()
	}
}

/// CSS `scrollbar-color`: the thumb and track colours of a scrollbar.
#[derive(Debug, Clone, Copy, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ScrollbarColor {
	/// The draggable thumb colour.
	pub thumb: Color,
	/// The track (groove) colour.
	pub track: Color,
}

impl AsCssValue for ScrollbarColor {
	fn as_css_value(&self) -> Result<CssValue> {
		// CSS `scrollbar-color: <thumb> <track>`
		format!(
			"{} {}",
			self.thumb.as_css_value()?.to_string(),
			self.track.as_css_value()?.to_string()
		)
		.xmap(CssValue::expression)
		.xok()
	}
}

/// Resolved scrollbar styling for a scroll container: thumb/track colours and
/// the width keyword. A renderer-agnostic value (the charcell scrollbar paint
/// reads it, the native renderer would too); only the glyph choice is
/// charcell-specific.
#[derive(Debug, Default, Clone, Copy, PartialEq, Component)]
pub struct ScrollbarStyle {
	/// `None` keeps the renderer's default thumb colour.
	pub thumb: Option<Color>,
	/// `None` keeps the renderer's default track colour.
	pub track: Option<Color>,
	pub width: ScrollbarWidth,
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
	/// Grid tracks, read when `display: grid`.
	pub grid: GridTracks,
	pub flex_order: i32,
	pub flex_grow: u32,
	pub align_self: AlignSelf,
	/// Horizontal overflow handling, mapping to CSS `overflow-x`.
	pub overflow_x: Overflow,
	/// Vertical overflow handling, mapping to CSS `overflow-y`.
	pub overflow_y: Overflow,
}

impl LayoutStyle {
	pub const DEFAULT: Self = Self {
		display: Display::Block,
		white_space: WhiteSpace::Normal,
		flex_box: FlexBox::row(),
		grid: GridTracks::DEFAULT,
		flex_order: 0,
		flex_grow: 0,
		align_self: AlignSelf::Auto,
		overflow_x: Overflow::Visible,
		overflow_y: Overflow::Visible,
	};

	/// Whether either axis clips its overflow.
	pub fn clips(&self) -> bool {
		self.overflow_x.is_clipped() || self.overflow_y.is_clipped()
	}

	/// Set both overflow axes at once (CSS `overflow` shorthand).
	pub fn overflow(mut self, overflow: Overflow) -> Self {
		self.overflow_x = overflow;
		self.overflow_y = overflow;
		self
	}

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

	/// Create a grid layout style with the given column count (12 is the
	/// conventional default) and square row tracks.
	pub fn grid(columns: u32) -> Self {
		Self {
			display: Display::Grid,
			grid: GridTracks {
				columns: GridColumns(columns),
				rows: GridRows::Square,
			},
			..default()
		}
	}

	pub fn grid_rows(mut self, rows: GridRows) -> Self {
		self.grid.rows = rows;
		self
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
