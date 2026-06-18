//! The BSX inline-style seam: the `bx:style` directive.
//!
//! Like the [`BsxTagResolvers`](crate::prelude::BsxTagResolvers) and
//! [`EventRegistry`](crate::prelude::EventRegistry) seams, core knows nothing
//! concrete here: it parses `bx:style`'s raw declaration text and source span,
//! then hands them to a downstream handler. The handler (registered by `beet_ui`,
//! where the style types live) parses the declarations into a one-off `Rule` and
//! attaches a unique, span-derived class to the element, the markup twin of the
//! Rust `inline_class!` macro.
//!
//! The seam exists because the declaration grammar and the `RuleSet` live in a
//! higher crate that core cannot reference, exactly as for `<Rule>`.

use super::ast::*;
use crate::prelude::*;
use alloc::sync::Arc;

/// The `bx:style` handler: parses the raw declaration `source` against the
/// element's [`FileSpan`] and mutates the build entity (register a one-off rule,
/// attach the minted inline class).
pub type StyleResolverFn = Arc<
	dyn Fn(&mut EntityWorldMut, &str, &FileSpan) -> Result<()> + Send + Sync,
>;

/// The `bx:style` seam: the single handler that lowers a `bx:style` directive's
/// declarations into a one-off [`Rule`] plus a span-derived class.
///
/// Consulted by the BSX resolver when an element carries a `bx:style` directive.
/// Empty by default: core registers no handler.
#[derive(Default, Resource)]
pub struct StyleResolver(Option<StyleResolverFn>);

impl StyleResolver {
	/// Set the `bx:style` handler, replacing any existing one.
	pub fn set(
		&mut self,
		handler: impl Fn(&mut EntityWorldMut, &str, &FileSpan) -> Result<()>
		+ Send
		+ Sync
		+ 'static,
	) -> &mut Self {
		self.0 = Some(Arc::new(handler));
		self
	}

	/// The registered handler, if any.
	pub fn get(&self) -> Option<StyleResolverFn> { self.0.clone() }
}

/// The raw declaration text and source span of an element's `bx:style` directive,
/// if it declares one.
pub fn bsx_style_attr(el: &BsxElement) -> Option<(&str, &FileSpan)> {
	el.attributes.iter().find_map(|attr| match &attr.value {
		AttrValue::Style { source, span } if attr.key == "bx:style" => {
			Some((source.as_str(), span))
		}
		_ => None,
	})
}
