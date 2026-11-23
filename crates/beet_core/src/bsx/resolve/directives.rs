//! The directive vocabulary the build walk still reads: which attribute keys
//! are directives rather than HTML attributes, and how a slot routing marker
//! reads. Every other directive is rewritten away before the walk
//! ([`BsxDirectives`]).

use crate::prelude::*;

/// The [`SlotChild`] routing marker for an element's `slot`, if any.
pub(super) fn slot_routing(el: &BsxElement) -> Option<SlotChild> {
	el.attributes.iter().find_map(|attr| {
		if !is_slot_directive(&attr.key) {
			return None;
		}
		match &attr.value {
			AttrValue::Str(name) => Some(SlotChild::named(name.clone())),
			_ => Some(SlotChild::new()),
		}
	})
}

/// Whether a key is the `slot` routing directive, HTML's own spelling.
pub(super) fn is_slot_directive(key: &str) -> bool { key == "slot" }

/// Whether a key is a `bx:` or slot directive rather than an HTML attribute.
pub(in crate::bsx) fn is_directive(key: &str) -> bool {
	key.starts_with("bx:") || is_slot_directive(key)
}
