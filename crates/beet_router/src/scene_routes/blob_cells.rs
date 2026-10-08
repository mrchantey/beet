//! [`BlobCells`], the addressed cells of any store file.
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_ui::prelude::*;

/// Request params for [`BlobCells`], surfaced in `--help`.
#[derive(Reflect)]
struct BlobCellsParams {
	/// The file's path in the store, ie `assets/forms/plan.docx`.
	path: RelPath,
}

/// `cells <path>`: the addressed cells of any file in the nearest ancestor
/// [`BlobStore`], parsed as its media type: every table cell, `t1r2c3`, in
/// document order, and every unlocked workbook cell, `Sheet!A1`, each with
/// what a reader reads in it. The map an edit addresses a form by, answered
/// as a table for a markup `Accept` and as its rows for a serde one.
///
/// ```bsx
/// <BlobCells/>
/// ```
///
/// ```sh
/// beet cells assets/forms/plan.docx --accept=text/markdown
/// ```
#[action(route = "cells/*path")]
#[derive(Default, Component, Reflect)]
#[reflect(Component, Default)]
#[require(ParamsPartial = ParamsPartial::new::<BlobCellsParams>())]
pub async fn BlobCells(
	cx: ActionContext<Request>,
) -> Result<DataPage<Vec<CellText>>> {
	let path = cx.input.parse_params::<BlobCellsParams>()?.path;
	let bytes = cx
		.caller
		.with_state::<AncestorQuery<&BlobStore>, _>(|entity, stores| {
			stores.get(entity).cloned()
		})
		.await??
		.blob(path)
		.get_media()
		.await?;
	let cells = cx
		.caller
		.world()
		.with(move |world: &mut World| -> Result<Vec<CellText>> {
			let root = world.spawn_empty().id();
			MediaParser::new().parse(ParseContext::new(
				&mut world.entity_mut(root),
				&bytes,
			))?;
			let cells =
				world.with_state::<TableCells, _>(|cells| cells.listing(root));
			world.entity_mut(root).despawn();
			cells.xok()
		})
		.await?;
	let scene = CellText::table(&cells);
	DataPage::new(&cx.caller, scene, cells).await
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	/// A markdown file's table cells, as a table for a person.
	#[beet_core::test]
	async fn lists_a_store_files_cells() {
		let store = BlobStore::temp();
		store
			.insert(
				&"forms/plan.md".into(),
				"| Name |  |\n|---|---|\n| Business \\| name | Acme |\n"
					.to_owned(),
			)
			.await
			.unwrap();
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		let root = world.spawn((store, Router, children![BlobCells])).flush();
		world
			.entity_mut(root)
			.exchange(
				Request::get("cells/forms/plan.md")
					.with_header::<header::Accept>(vec![MediaType::Markdown]),
			)
			.await
			.unwrap_str()
			.await
			.xpect_eq(
				"| Cell | Text |\n|---|---|\n| t1r1c1 | Name |\n| t1r1c2 | (empty) |\n\
				 | t1r2c1 | Business \\| name |\n| t1r2c2 | Acme |\n",
			);
	}
}
