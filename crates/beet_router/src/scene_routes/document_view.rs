//! [`DocumentView`], any document in the store a request names, rendered as
//! it accepts.
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// `view <path>`: any document in the nearest ancestor [`BlobStore`], a file
/// beet can parse, parsed as its media type and rendered as the request
/// accepts, exactly as a [`BlobPage`] serves its one file, so a markdown
/// page, a Word file or a slide deck answers as markdown, text or HTML.
///
/// ```bsx
/// <DocumentView/>
/// ```
///
/// ```sh
/// beet view docs/plan.docx --accept=text/markdown
/// ```
#[action(route = "view/*file")]
#[derive(Default, Component, Reflect)]
#[reflect(Component, Default)]
pub async fn DocumentView(
	/// The file's path in the store, ie `docs/plan.docx`.
	#[param]
	file: RelPath,
	cx: ActionContext<Request>,
) -> Result<PageRequest> {
	DocumentView::page(&cx.caller, cx.input.parts().clone(), file).await
}

impl DocumentView {
	/// The document at `file` in the store above `caller` parsed into a
	/// fresh render root for the request `parts`, its root declarations
	/// seeded, as [`BlobPage`] parses its file: the one parse every view of a
	/// store document goes through.
	pub(crate) async fn page(
		caller: &AsyncEntity,
		parts: RequestParts,
		file: RelPath,
	) -> Result<PageRequest> {
		let blob = caller
			.with_state::<AncestorQuery<&BlobStore>, _>(|entity, stores| {
				stores.get(entity).cloned()
			})
			.await??
			.blob(file);
		BlobPage::serve(caller, parts, blob).await
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;
	#[cfg(feature = "markdown_parser")]
	use beet_ui::prelude::*;

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
		let root = world
			.spawn((store, Router, children![DocumentView]))
			.flush();
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

	/// Stands in for a component a downstream crate declares frontmatter.
	#[cfg(feature = "markdown_parser")]
	#[derive(
		Debug,
		Default,
		Clone,
		PartialEq,
		Component,
		Reflect,
		serde::Serialize,
		serde::Deserialize,
	)]
	#[reflect(Component, Default, Frontmatter)]
	struct Course {
		unit: Option<SmolStr>,
		weeks: u32,
	}

	/// A component declared outside beet_ui round-trips through a view asked
	/// for frontmatter: its section declares it on the root, and the view
	/// writes it back as its table beside the page's own keys.
	#[cfg(feature = "markdown_parser")]
	#[beet_core::test]
	async fn views_a_declared_component_as_its_section() {
		let store = BlobStore::temp();
		store
			.insert(
				&"docs/plan.md".into(),
				"+++\ntitle = \"Plan\"\n\n[Course]\nunit = \"BSBOPS404\"\nweeks = 12\n+++\n\n# Plan\n"
					.to_owned(),
			)
			.await
			.unwrap();
		let mut app = App::new();
		app.add_plugins((AsyncPlugin, RouterPlugin))
			.register_type::<Course>();
		let world = app.world_mut();
		let root = world
			.spawn((store, Router, children![DocumentView]))
			.flush();
		world
			.entity_mut(root)
			.exchange(
				Request::get("view/docs/plan.md")
					.with_param("frontmatter", "toml")
					.with_header::<header::Accept>(vec![MediaType::Markdown]),
			)
			.await
			.unwrap_str()
			.await
			.xpect_eq(
				"+++\ntitle = \"Plan\"\n\n[Course]\nunit = \"BSBOPS404\"\nweeks = 12\n+++\n\n# Plan\n",
			);
	}
}
