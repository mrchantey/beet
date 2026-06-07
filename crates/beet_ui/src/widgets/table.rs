//! `Table` widget — a `<table>` with `head`, default, and `foot` slots.
//!
//! Slot content is supplied as `<tr>` rows; the head/foot slots wrap their
//! content in `<thead>`/`<tfoot>` automatically.
use crate::token::Classes;
use crate::style::material::classes;
use beet_core::prelude::*;

/// A styled `<table>` with semantic head/body/foot sections.
///
/// Slots: `head` (one or more `<tr>` for `<thead>`), default (rows for
/// `<tbody>`), `foot` (rows for `<tfoot>`).
///
/// Set `vertical_lines` for a full cell grid (vertical dividers as well as the
/// default horizontal row rules).
#[scene]
pub fn Table(#[prop] vertical_lines: bool) -> impl Scene {
	let mut class_set = Classes::new([classes::TABLE]);
	if vertical_lines {
		class_set.insert_class(classes::TABLE_VERTICAL_BORDERS);
	}
	rsx! {
		<table {class_set}>
			<thead>
				<slot name="head"/>
			</thead>
			<tbody>
				<slot/>
			</tbody>
			<tfoot>
				<slot name="foot"/>
			</tfoot>
		</table>
	}
}
