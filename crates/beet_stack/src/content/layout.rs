//! Interface-agnostic layout primitives.
//!
//! Provides basic flexbox-like layout types that work across
//! rendering backends (ratatui, bevy_ui, DOM). Units are limited
//! to [`Em`] and [`Percent`] to stay agnostic to integer-based (TUI)
//! and float-based (DOM) rendering.
use beet_core::prelude::*;

/// Layout direction for flex containers.
#[derive(
	Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Reflect, Component,
)]
#[reflect(Component)]
pub enum FlexDirection {
	/// Stack children vertically (default for most content).
	#[default]
	Column,
	/// Stack children horizontally.
	Row,
}

/// A length expressed in `em` units, relative to the
/// current font size.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Reflect)]
pub struct Em(pub f32);

impl Em {
	pub fn new(value: f32) -> Self { Self(value) }
}

/// A length expressed as a percentage of the parent dimension.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Reflect)]
pub struct Percent(pub f32);

impl Percent {
	pub fn new(value: f32) -> Self { Self(value) }
}

/// A dimension that can be expressed in either [`Em`] or [`Percent`] units.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Reflect)]
pub enum Unit {
	/// Relative to the current font size.
	Em(f32),
	/// Percentage of the parent dimension.
	Percent(f32),
}

impl From<Em> for Unit {
	fn from(em: Em) -> Self { Unit::Em(em.0) }
}

impl From<Percent> for Unit {
	fn from(pct: Percent) -> Self { Unit::Percent(pct.0) }
}

/// How a flex item should grow or shrink relative to siblings.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Reflect)]
pub struct FlexGrow(pub f32);

impl Default for FlexGrow {
	fn default() -> Self { Self(1.0) }
}

/// Spacing around an element, expressed in [`Unit`].
#[derive(Debug, Default, Clone, Copy, PartialEq, PartialOrd, Reflect)]
pub struct Spacing {
	pub top: Option<Unit>,
	pub right: Option<Unit>,
	pub bottom: Option<Unit>,
	pub left: Option<Unit>,
}

impl Spacing {
	/// Uniform spacing on all sides.
	pub fn all(unit: Unit) -> Self {
		Self {
			top: Some(unit),
			right: Some(unit),
			bottom: Some(unit),
			left: Some(unit),
		}
	}
	/// Symmetric spacing: vertical (top/bottom) and horizontal (left/right).
	pub fn symmetric(vertical: Unit, horizontal: Unit) -> Self {
		Self {
			top: Some(vertical),
			right: Some(horizontal),
			bottom: Some(vertical),
			left: Some(horizontal),
		}
	}
}

/// Layout component describing how an entity arranges its children
/// and occupies space.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component)]
pub struct Layout {
	/// Direction children are stacked.
	pub direction: FlexDirection,
	/// How this element grows relative to siblings.
	pub flex_grow: FlexGrow,
	/// Optional fixed or proportional width.
	pub width: Option<Unit>,
	/// Optional fixed or proportional height.
	pub height: Option<Unit>,
	/// Padding inside the element.
	pub padding: Spacing,
	/// Margin outside the element.
	pub margin: Spacing,
}

impl Layout {
	pub fn row() -> Self {
		Self {
			direction: FlexDirection::Row,
			..Default::default()
		}
	}
	pub fn column() -> Self {
		Self {
			direction: FlexDirection::Column,
			..Default::default()
		}
	}
	pub fn with_width(mut self, width: impl Into<Unit>) -> Self {
		self.width = Some(width.into());
		self
	}
	pub fn with_height(mut self, height: impl Into<Unit>) -> Self {
		self.height = Some(height.into());
		self
	}
	pub fn with_padding(mut self, padding: Spacing) -> Self {
		self.padding = padding;
		self
	}
	pub fn with_margin(mut self, margin: Spacing) -> Self {
		self.margin = margin;
		self
	}
	pub fn with_flex_grow(mut self, grow: f32) -> Self {
		self.flex_grow = FlexGrow(grow);
		self
	}
}
