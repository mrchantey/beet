//! The directive vocabulary: which attribute keys are `bx:`/slot directives
//! rather than HTML attributes, and how a slot routing marker reads.

use crate::prelude::*;

/// The [`SlotChild`] routing marker for an element's `slot`/`bx:slot`, if any.
pub(super) fn slot_routing(el: &BsxElement) -> Option<SlotChild> {
	el.attributes.iter().find_map(|attr| {
		if attr.key != "slot" && attr.key != "bx:slot" {
			return None;
		}
		match &attr.value {
			AttrValue::Str(name) => Some(SlotChild::named(name.clone())),
			_ => Some(SlotChild::new()),
		}
	})
}

/// Whether a key is a `bx:`/slot directive rather than an HTML attribute.
pub(in crate::bsx) fn is_directive(key: &str) -> bool {
	key.starts_with("bx:") || key == "slot"
}

/// The `bx:` directives with dedicated structural meaning, as opposed to a
/// `bx:<event>` verb trigger. Anything else under `bx:` is treated as an event
/// (resolved through the [`EventRegistry`], a graceful no-op when unregistered).
const STRUCTURAL_DIRECTIVES: &[&str] = &[
	"bx:cfg",
	"bx:scope",
	"bx:for",
	"bx:key",
	"bx:slot",
	"bx:ref",
	"bx:schema",
	"bx:style",
];

/// Whether a key is a `bx:<event>` verb-trigger directive (eg `bx:click`), ie a
/// `bx:` key that is not one of the [`STRUCTURAL_DIRECTIVES`].
pub(in crate::bsx) fn is_event_directive(key: &str) -> bool {
	key.starts_with("bx:") && !STRUCTURAL_DIRECTIVES.contains(&key)
}
