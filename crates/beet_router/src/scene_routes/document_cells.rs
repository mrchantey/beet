//! [`DocumentCells`], the addressed cells of any document in the store.
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_ui::prelude::*;

/// `cells <path>`: the addressed cells of any document in the nearest
/// ancestor [`BlobStore`], parsed through the one parse [`DocumentView`]
/// renders, root declarations and all: every table cell, `t1r2c3`, in
/// document order, and every unlocked workbook cell, `Sheet!A1`, each with
/// what a reader reads in it. The map an edit addresses a form by, answered
/// as a table for a markup `Accept` and as its rows for a serde one.
///
/// ```bsx
/// <Route path="document"><DocumentCells/></Route>
/// ```
///
/// ```sh
/// beet document/cells assets/forms/plan.docx --accept=text/markdown
/// ```
#[action(route = "cells/*file")]
#[derive(Default, Component, Reflect)]
#[reflect(Component, Default)]
pub async fn DocumentCells(
	/// The file's path in the store, ie `assets/forms/plan.docx`.
	#[param]
	file: RelPath,
	cx: ActionContext<Request>,
) -> Result<DataPage<Vec<CellText>>> {
	let PageRequest(root) =
		DocumentView::page(&cx.caller, cx.input.parts().clone(), file).await?;
	let cells = cx
		.caller
		.world()
		.with(move |world: &mut World| {
			let cells = CellText::listing(world, root);
			world.entity_mut(root).despawn();
			cells
		})
		.await;
	let scene = CellText::table(&cells);
	DataPage::new(&cx.caller, scene, cells).await
}

// a markdown table has cells only where markdown parses
#[cfg(all(test, feature = "markdown_parser"))]
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
		let root = world
			.spawn((store, Router, children![DocumentCells]))
			.flush();
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
