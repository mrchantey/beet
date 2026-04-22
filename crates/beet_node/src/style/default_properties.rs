use crate::prelude::*;
use crate::style::*;
use beet_core::prelude::*;

/// A set of default properties applied to elements matching the given criteria.
pub struct DefaultPropertySet {
	/// Element tags to match; empty means match any tag.
	include_tags: Vec<SmolStr>,
	exclude_tags: Vec<SmolStr>,
	/// Attribute keys to match, with optional value constraint.
	include_attributes: Vec<(SmolStr, Option<Value>)>,
	exclude_attributes: Vec<(SmolStr, Option<Value>)>,
	property_map: PropertyMap,
}

impl DefaultPropertySet {
	/// Match elements with the given tag.
	pub fn new(tag: impl Into<SmolStr>) -> Self {
		Self {
			include_tags: vec![tag.into()],
			exclude_tags: Vec::new(),
			include_attributes: Vec::new(),
			exclude_attributes: Vec::new(),
			property_map: PropertyMap::default(),
		}
	}

	/// Match any element regardless of tag.
	pub fn any() -> Self {
		Self {
			include_tags: Vec::new(),
			exclude_tags: Vec::new(),
			include_attributes: Vec::new(),
			exclude_attributes: Vec::new(),
			property_map: PropertyMap::default(),
		}
	}

	/// Add a property mapped to a token.
	pub fn with(mut self, property: impl Into<Property>, value: Token) -> Self {
		self.property_map.insert(property.into(), value);
		self
	}

	/// Returns true if the element satisfies all tag and attribute criteria.
	pub fn passes(&self, el: &ElementView) -> bool {
		self.passes_tags(el.element) && self.passes_attributes(el)
	}

	pub fn passes_tags(&self, el: &Element) -> bool {
		(self.include_tags.is_empty()
			|| self.include_tags.iter().any(|tag| tag == el.tag()))
			&& !self.exclude_tags.iter().any(|tag| tag == el.tag())
	}

	pub fn passes_attributes(&self, el: &ElementView) -> bool {
		(self.include_attributes.is_empty()
			|| self.include_attributes.iter().any(|(key, val)| match val {
				Some(expected) => {
					el.attribute(key).map(|v| v == expected).unwrap_or(false)
				}
				None => el.attribute(key).is_some(),
			})) && !self.exclude_attributes.iter().any(|(key, val)| match val {
			Some(expected) => {
				el.attribute(key).map(|v| v == expected).unwrap_or(false)
			}
			None => el.attribute(key).is_some(),
		})
	}

	pub fn property_map(&self) -> &PropertyMap { &self.property_map }
}

/// A collection of [`DefaultPropertySet`] rules applied before user styles.
#[derive(Resource, Component)]
pub struct DefaultPropertyMap(Vec<DefaultPropertySet>);

impl DefaultPropertyMap {
	pub fn new(sets: Vec<DefaultPropertySet>) -> Self { Self(sets) }

	pub fn iter(&self) -> impl Iterator<Item = &DefaultPropertySet> {
		self.0.iter()
	}

	/// Returns a merged [`PropertyMap`] for `el`, applying all matching sets
	/// in order (later sets overwrite earlier ones for the same property).
	pub fn resolve(&self, el: &ElementView) -> PropertyMap {
		let mut map = PropertyMap::default();
		for set in self.0.iter() {
			if set.passes(el) {
				map.merge(set.property_map().clone());
			}
		}
		map
	}
}

/// Material Design 3 baseline defaults for common HTML elements.
///
/// Provides sensible typography, color, shape, and elevation defaults so that
/// unstyled content renders with a clean MD3 appearance.
pub fn baseline_default_properties() -> DefaultPropertyMap {
	DefaultPropertyMap::new(vec![
		// ── Body ─────────────────────────────────────────────────────────────
		// Base surface colors and plain body typography.
		DefaultPropertySet::new("body")
			.with(props::FOREGROUND_COLOR, colors::ON_BACKGROUND)
			.with(props::BACKGROUND_COLOR, colors::BACKGROUND)
			.with(props::FONT, TYPEFACE_PLAIN)
			.with(props::FONT_SIZE, FONT_SIZE_BODY_LARGE)
			.with(props::FONT_WEIGHT, WEIGHT_REGULAR)
			.with(props::LINE_HEIGHT, LINE_HEIGHT_BODY_LARGE),
		// ── Headings ─────────────────────────────────────────────────────────
		// h1 → Display Large (brand typeface)
		DefaultPropertySet::new("h1")
			.with(props::FOREGROUND_COLOR, colors::ON_SURFACE)
			.with(props::FONT, TYPEFACE_BRAND)
			.with(props::FONT_SIZE, FONT_SIZE_DISPLAY_LARGE)
			.with(props::FONT_WEIGHT, WEIGHT_REGULAR)
			.with(props::LINE_HEIGHT, LINE_HEIGHT_DISPLAY_LARGE),
		// h2 → Display Medium
		DefaultPropertySet::new("h2")
			.with(props::FOREGROUND_COLOR, colors::ON_SURFACE)
			.with(props::FONT, TYPEFACE_BRAND)
			.with(props::FONT_SIZE, FONT_SIZE_DISPLAY_MEDIUM)
			.with(props::FONT_WEIGHT, WEIGHT_REGULAR)
			.with(props::LINE_HEIGHT, LINE_HEIGHT_DISPLAY_MEDIUM),
		// h3 → Display Small
		DefaultPropertySet::new("h3")
			.with(props::FOREGROUND_COLOR, colors::ON_SURFACE)
			.with(props::FONT, TYPEFACE_BRAND)
			.with(props::FONT_SIZE, FONT_SIZE_DISPLAY_SMALL)
			.with(props::FONT_WEIGHT, WEIGHT_REGULAR)
			.with(props::LINE_HEIGHT, LINE_HEIGHT_DISPLAY_SMALL),
		// h4 → Headline Large
		DefaultPropertySet::new("h4")
			.with(props::FOREGROUND_COLOR, colors::ON_SURFACE)
			.with(props::FONT, TYPEFACE_PLAIN)
			.with(props::FONT_SIZE, FONT_SIZE_HEADLINE_LARGE)
			.with(props::FONT_WEIGHT, WEIGHT_REGULAR)
			.with(props::LINE_HEIGHT, LINE_HEIGHT_HEADLINE_LARGE),
		// h5 → Headline Medium
		DefaultPropertySet::new("h5")
			.with(props::FOREGROUND_COLOR, colors::ON_SURFACE)
			.with(props::FONT, TYPEFACE_PLAIN)
			.with(props::FONT_SIZE, FONT_SIZE_HEADLINE_MEDIUM)
			.with(props::FONT_WEIGHT, WEIGHT_REGULAR)
			.with(props::LINE_HEIGHT, LINE_HEIGHT_HEADLINE_MEDIUM),
		// h6 → Headline Small
		DefaultPropertySet::new("h6")
			.with(props::FOREGROUND_COLOR, colors::ON_SURFACE)
			.with(props::FONT, TYPEFACE_PLAIN)
			.with(props::FONT_SIZE, FONT_SIZE_HEADLINE_SMALL)
			.with(props::FONT_WEIGHT, WEIGHT_REGULAR)
			.with(props::LINE_HEIGHT, LINE_HEIGHT_HEADLINE_SMALL),
		// ── Body text ─────────────────────────────────────────────────────────
		DefaultPropertySet::new("p")
			.with(props::FOREGROUND_COLOR, colors::ON_SURFACE)
			.with(props::FONT, TYPEFACE_PLAIN)
			.with(props::FONT_SIZE, FONT_SIZE_BODY_LARGE)
			.with(props::FONT_WEIGHT, WEIGHT_REGULAR)
			.with(props::LINE_HEIGHT, LINE_HEIGHT_BODY_LARGE),
		DefaultPropertySet::new("blockquote")
			.with(props::FOREGROUND_COLOR, colors::ON_SURFACE_VARIANT)
			.with(props::FONT, TYPEFACE_PLAIN)
			.with(props::FONT_SIZE, FONT_SIZE_BODY_LARGE)
			.with(props::LINE_HEIGHT, LINE_HEIGHT_BODY_LARGE),
		DefaultPropertySet::new("small")
			.with(props::FONT_SIZE, FONT_SIZE_BODY_SMALL)
			.with(props::LINE_HEIGHT, LINE_HEIGHT_BODY_SMALL),
		// ── Links ─────────────────────────────────────────────────────────────
		// Primary color; text-decoration left to CSS defaults.
		DefaultPropertySet::new("a")
			.with(props::FOREGROUND_COLOR, colors::PRIMARY),
		// ── Buttons ───────────────────────────────────────────────────────────
		// Filled button: primary surface, label-large type, pill shape.
		DefaultPropertySet::new("button")
			.with(props::FOREGROUND_COLOR, colors::ON_PRIMARY)
			.with(props::BACKGROUND_COLOR, colors::PRIMARY)
			.with(props::FONT, TYPEFACE_PLAIN)
			.with(props::FONT_SIZE, FONT_SIZE_LABEL_LARGE)
			.with(props::FONT_WEIGHT, WEIGHT_MEDIUM)
			.with(props::LINE_HEIGHT, LINE_HEIGHT_LABEL_LARGE)
			.with(props::SHAPE, SHAPE_FULL)
			.with(props::ELEVATION, ELEVATION_1),
		// ── Form elements ─────────────────────────────────────────────────────
		DefaultPropertySet::new("input")
			.with(props::FOREGROUND_COLOR, colors::ON_SURFACE)
			.with(props::BACKGROUND_COLOR, colors::SURFACE_CONTAINER)
			.with(props::FONT, TYPEFACE_PLAIN)
			.with(props::FONT_SIZE, FONT_SIZE_BODY_LARGE)
			.with(props::FONT_WEIGHT, WEIGHT_REGULAR)
			.with(props::SHAPE, SHAPE_EXTRA_SMALL)
			.with(props::OUTLINE_WIDTH, OUTLINE_WIDTH_THIN),
		DefaultPropertySet::new("textarea")
			.with(props::FOREGROUND_COLOR, colors::ON_SURFACE)
			.with(props::BACKGROUND_COLOR, colors::SURFACE_CONTAINER)
			.with(props::FONT, TYPEFACE_PLAIN)
			.with(props::FONT_SIZE, FONT_SIZE_BODY_LARGE)
			.with(props::FONT_WEIGHT, WEIGHT_REGULAR)
			.with(props::SHAPE, SHAPE_EXTRA_SMALL)
			.with(props::OUTLINE_WIDTH, OUTLINE_WIDTH_THIN),
		DefaultPropertySet::new("select")
			.with(props::FOREGROUND_COLOR, colors::ON_SURFACE)
			.with(props::BACKGROUND_COLOR, colors::SURFACE_CONTAINER)
			.with(props::FONT, TYPEFACE_PLAIN)
			.with(props::FONT_SIZE, FONT_SIZE_BODY_LARGE)
			.with(props::SHAPE, SHAPE_EXTRA_SMALL)
			.with(props::OUTLINE_WIDTH, OUTLINE_WIDTH_THIN),
		// Label Medium, subdued on-surface-variant color.
		DefaultPropertySet::new("label")
			.with(props::FOREGROUND_COLOR, colors::ON_SURFACE_VARIANT)
			.with(props::FONT, TYPEFACE_PLAIN)
			.with(props::FONT_SIZE, FONT_SIZE_LABEL_MEDIUM)
			.with(props::FONT_WEIGHT, WEIGHT_MEDIUM)
			.with(props::LINE_HEIGHT, LINE_HEIGHT_LABEL_MEDIUM),
		// ── Code ─────────────────────────────────────────────────────────────
		// Inline code — monospace, slightly subdued.
		DefaultPropertySet::new("code")
			.with(props::FOREGROUND_COLOR, colors::ON_SURFACE_VARIANT)
			.with(props::FONT, TYPEFACE_MONO)
			.with(props::FONT_SIZE, FONT_SIZE_BODY_MEDIUM),
		// Code block — surface-container background, monospace.
		DefaultPropertySet::new("pre")
			.with(props::FOREGROUND_COLOR, colors::ON_SURFACE_VARIANT)
			.with(props::BACKGROUND_COLOR, colors::SURFACE_CONTAINER)
			.with(props::FONT, TYPEFACE_MONO)
			.with(props::FONT_SIZE, FONT_SIZE_BODY_MEDIUM)
			.with(props::SHAPE, SHAPE_EXTRA_SMALL),
	])
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn baseline_compiles_and_has_entries() {
		let map = baseline_default_properties();
		// Confirm a non-empty rule set is produced.
		(map.iter().count() > 0).xpect_true();
	}

	#[test]
	fn passes_tags_matches_correctly() {
		let set = DefaultPropertySet::new("h1");
		let h1 = Element::new("h1");
		let p = Element::new("p");
		set.passes_tags(&h1).xpect_true();
		set.passes_tags(&p).xpect_false();
	}

	#[test]
	fn any_matches_all_tags() {
		let set = DefaultPropertySet::any();
		let div = Element::new("div");
		set.passes_tags(&div).xpect_true();
	}
}
