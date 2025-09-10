use beet_rsx::prelude::*;
use bevy::prelude::*;


/// A table
///
/// ## Slots
/// - `head`: The table header, which should contain `<tr>` elements.
/// - `foot`: The table footer, which should contain `<tr>` elements.
/// - `default`: The body of the table, which should contain `<tr>` elements.
///	## Example
///
/// ```
/// # use beet_rsx::prelude::*;
///
///
/// ```
#[template]
pub fn Table() -> impl Bundle {
	rsx! {
		<table class="bt-c-table">
			<thead>
				<slot name="head" />
			</thead>
			<tbody>
				<slot />
			</tbody>
			<tfoot>
				<slot name="foot" />
			</tfoot>
		</table>
		<style src="./table.css" />
	}
}
