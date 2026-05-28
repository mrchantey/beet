//! `Table` widget — a `<table>` with `head`, default, and `foot` slots.
//!
//! Slot content is supplied as `<tr>` rows; the head/foot slots wrap their
//! content in `<thead>`/`<tfoot>` automatically.
use beet_core::prelude::*;

/// A styled `<table>` with semantic head/body/foot sections.
///
/// Slots: `head` (one or more `<tr>` for `<thead>`), default (rows for
/// `<tbody>`), `foot` (rows for `<tfoot>`).
#[scene]
pub fn Table() -> impl Scene {
	rsx! {
		<table {Classes::new(["table"])}>
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
