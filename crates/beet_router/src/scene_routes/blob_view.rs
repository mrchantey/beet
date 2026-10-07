//! [`BlobView`], any store file a request names, rendered as it accepts.
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Request params for [`BlobView`], surfaced in `--help`.
#[derive(Reflect)]
struct BlobViewParams {
	/// The file's path in the store, ie `docs/plan.docx`.
	path: RelPath,
}

/// `view <path>`: any file in the nearest ancestor [`BlobStore`], parsed as
/// its media type and rendered as the request accepts, exactly as a
/// [`BlobPage`] serves its one file, so a markdown page, a Word file or a
/// slide deck answers as markdown, text or HTML.
///
/// ```bsx
/// <BlobView/>
/// ```
///
/// ```sh
/// beet view docs/plan.docx --accept=text/markdown
/// ```
#[action(route = "view/*path")]
#[derive(Default, Component, Reflect)]
#[reflect(Component, Default)]
#[require(ParamsPartial = ParamsPartial::new::<BlobViewParams>())]
pub async fn BlobView(cx: ActionContext<Request>) -> Result<PageRequest> {
	let path = cx.input.parse_params::<BlobViewParams>()?.path;
	let blob = cx
		.caller
		.with_state::<AncestorQuery<&BlobStore>, _>(|entity, stores| {
			stores.get(entity).cloned()
		})
		.await??
		.blob(path);
	BlobPage::serve(&cx.caller, cx.input.parts().clone(), blob).await
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	/// A file a request names is parsed by its media type and rendered as the
	/// request accepts.
	#[beet_core::test]
	async fn views_a_store_file_as_accepted() {
		let store = BlobStore::temp();
		store
			.insert(
				&"docs/plan.html".into(),
				"<h1>Plan</h1><table><tr><th>a</th><th>b</th></tr><tr><td>c</td><td>d</td></tr></table>"
					.to_owned(),
			)
			.await
			.unwrap();
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		let root = world.spawn((store, Router, children![BlobView])).flush();
		let mut view = async |accept: MediaType| {
			world
				.entity_mut(root)
				.exchange(
					Request::get("view/docs/plan.html")
						.with_header::<header::Accept>(vec![accept]),
				)
				.await
				.unwrap_str()
				.await
		};
		view(MediaType::Markdown)
			.await
			.xpect_eq("# Plan\n\n| a | b |\n|---|---|\n| c | d |\n");
		view(MediaType::Html).await.xpect_contains("<td>d</td>");
	}
}
