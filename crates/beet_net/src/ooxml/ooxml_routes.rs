//! The cells dump, the one view of an Office file that is not prose.
use crate::prelude::*;
use beet_core::prelude::*;

/// Registers [`OoxmlCells`], so an entry names it by tag.
#[derive(Default)]
pub struct OoxmlPlugin;

impl Plugin for OoxmlPlugin {
	fn build(&self, app: &mut App) { app.register_type::<OoxmlCells>(); }
}

/// Request params for [`OoxmlCells`], surfaced in `--help`.
#[derive(Reflect)]
struct CellsParams {
	/// The file's path in the store, ie `assets/forms/plan.docx`.
	path: RelPath,
}

/// `cells <path>`: every table cell of a Word file, `| t1r1c1 | text |`, or
/// every unlocked cell of a workbook, `| Sheet!A1 | value |`, one row each,
/// `(empty)` for nothing, read from the nearest ancestor [`BlobStore`]: the
/// map a fill is written against.
///
/// ```bsx
/// <Route path="ooxml"><OoxmlCells/></Route>
/// ```
///
/// ```sh
/// beet ooxml/cells assets/forms/plan.docx
/// ```
#[action]
#[derive(Component, Reflect)]
#[reflect(Component)]
#[require(
	PathPartial = PathPartial::new("cells/*path"),
	ParamsPartial = ParamsPartial::new::<CellsParams>()
)]
pub async fn OoxmlCells(cx: ActionContext<Request>) -> Result<Response> {
	let path = cx.input.parse_params::<CellsParams>()?.path;
	let blob = cx
		.caller
		.with_state::<AncestorQuery<&BlobStore>, _>(|entity, query| {
			query.get(entity).cloned()
		})
		.await??
		.blob(path.clone());
	let rows = match MediaType::from_path(path.as_str()) {
		MediaType::Docx => WordDocument::open(&blob)
			.await?
			.cells()
			.iter()
			.map(ToString::to_string)
			.collect::<Vec<_>>(),
		MediaType::Xlsx => Workbook::open(&blob)
			.await?
			.unlocked_cells()?
			.iter()
			.map(ToString::to_string)
			.collect(),
		_ => bevybail!(
			"`{path}` has no cells: expected a Word file or a workbook"
		),
	};
	Response::ok_text(format!("{}\n", rows.join("\n"))).xok()
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_action::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	async fn dumps_cells_from_the_store() {
		let store = BlobStore::temp();
		let mut form = WordDocument::from_body(
			"<w:tbl><w:tr><w:tc><w:p><w:r><w:t>Name</w:t></w:r></w:p></w:tc></w:tr></w:tbl>",
		)
		.unwrap();
		form.save(&store.blob(RelPath::new("forms/a form.docx")))
			.await
			.unwrap();
		let mut world = AsyncPlugin::world();
		let root = world.spawn((store, children![OoxmlCells])).id();
		let route = world.entity(root).get::<Children>().unwrap()[0];
		world
			.entity_mut(route)
			.call::<Request, Response>(
				Request::get("/")
					.with_param("path", "forms")
					.with_param("path", "a form.docx"),
			)
			.await
			.unwrap()
			.unwrap_str()
			.await
			.xpect_eq("| t1r1c1 | Name |\n");
	}
}
