//! RSX snippet extraction from Markdown source files.
//!
//! This module handles parsing Markdown and MDX files to extract RSX content
//! and spawn them as [`CombinatorTokens`] entities for further processing.

use crate::prelude::*;
use beet_core::prelude::*;
use beet_dom::prelude::*;
use beet_parse::prelude::*;
use quote::ToTokens;

/// Extracts RSX snippets from Markdown source files.
///
/// For each Markdown file (`.md` or `.mdx` extension), this system:
/// 1. Parses the Markdown content to valid RSX combinator syntax
/// 2. Spawns a child entity with [`CombinatorTokens`]
/// 3. If a [`MetaType`] is specified in an ancestor, extracts and parses
///    frontmatter metadata as a child [`NodeExpr`]
pub(crate) fn import_rsx_snippets_md(
	mut commands: Commands,
	query: Populated<(Entity, &SourceFile), Added<SourceFile>>,
	parents: Query<&ChildOf>,
	meta_types: Query<&MetaType>,
) -> Result {
	for (entity, path) in query.iter() {
		if let Some(ex) = path.extension()
		// TODO md should parse html only
			&& (ex == "md" || ex == "mdx")
		{
			trace!("markdown source file changed: {}", path.display());

			let file = fs_ext::read_to_string(path)?;
			let rsx_str = ParseMarkdown::markdown_to_rsx_str(&file);

			let mut snippet = commands.spawn((
				SnippetRoot::new(path.into_ws_path()?, LineCol::default()),
				StaticRoot,
				ChildOf(entity),
				CombinatorTokens::new(rsx_str),
			));


			if let Some(meta_type) = parents
				.iter_ancestors_inclusive(entity)
				.find_map(|e| meta_types.get(e).ok())
				&& let Some(meta_block) =
					ParseMarkdown::markdown_to_frontmatter_tokens(&file)?
			{
				let meta_type = &meta_type.inner();
				let err_msg = format!(
					"Failed to parse frontmatter into {}",
					meta_type.to_token_stream().to_string(),
				);
				// snippet roots are always fragments
				snippet.with_child(NodeExpr::new(syn::parse_quote! {{
					let meta: #meta_type = #meta_block.expect(#err_msg);
					meta
				}}));
			}
		}
	}
	Ok(())
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_rsx::prelude::*;

	#[test]
	fn parse_md() {
		let mut world = BuildPlugin::world();

		let entity = world
			.spawn(SourceFile::new(
				WsPathBuf::new("tests/test_site/test_docs/hello.md").into_abs(),
			))
			.id();

		world.run_schedule(ParseSourceFiles);
		let child = world.entity(entity).get::<Children>().unwrap()[0];
		world
			.run_system_cached_with(render_fragment, child)
			.unwrap()
			// only the output of the snippet, not the instance
			.xpect_eq("<h1>Hello</h1><p>This page is all about saying</p><main>## Nested Heading\n\tnested markdown doesnt work yet</main>");
	}
	#[test]
	fn parse_mdx() {
		let mut world = BuildPlugin::world();
		let entity = world
			.spawn(SourceFile::new(
				WsPathBuf::new("tests/test_site/test_docs/index.mdx")
					.into_abs(),
			))
			.id();

		world.run_schedule(ParseSourceFiles);
		let child = world.entity(entity).get::<Children>().unwrap()[0];
		world
			.run_system_cached_with(render_fragment, child)
			.unwrap()
			// only the output of the snippet, not the instance
			.xpect_eq(
				"<h1>Docs</h1><p>Docs are good for your health</p><div>1 + 1 is</div>",
			);
	}
}
