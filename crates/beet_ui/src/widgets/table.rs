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


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// Render a scene to plain charcell with the Material rule set.
	fn render_charcell(scene: impl Scene) -> String {
		let mut world = (
			bevy::app::TaskPoolPlugin::default(),
			bevy::asset::AssetPlugin::default(),
			bevy::scene::ScenePlugin,
			CharcellPlugin,
			crate::style::material::MaterialStylePlugin::default(),
		)
			.into_world();
		let root = world.spawn_scene(scene).unwrap().id();
		world.entity_mut(root).insert(FlexBuffer::new(40));
		world.run_schedule(crate::parse::PostParseTree);
		world.entity_mut(root).take::<FlexBuffer>().unwrap().render_plain()
	}

	fn demo(vertical_lines: bool) -> impl Scene {
		rsx! {
			<Table vertical_lines=vertical_lines>
				<tr slot="head"><th>"Name"</th><th>"Age"</th></tr>
				<tr><td>"Alice"</td><td>"30"</td></tr>
			</Table>
		}
	}

	/// The `vertical_lines` variant draws internal column dividers (`│`) on the
	/// terminal too, not just the web: the charcell cascade can't express the
	/// ancestor-scoped sibling rule, so `apply_table_vertical_borders` adds them.
	#[beet_core::test]
	fn vertical_lines_draw_column_dividers() {
		render_charcell(demo(true)).xpect_contains("│");
	}

	/// A default table has only horizontal row rules, no column dividers.
	#[beet_core::test]
	fn default_table_has_no_column_dividers() {
		render_charcell(demo(false)).xnot().xpect_contains("│");
	}
}
