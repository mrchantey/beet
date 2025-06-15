use crate::prelude::*;
use beet_router::prelude::HttpMethod;
use beet_router::types::RouteInfo;
use beet_router::types::RoutePath;
use beet_utils::prelude::ReadFile;
use beet_utils::prelude::WorkspacePathBuf;
use bevy::prelude::*;
use proc_macro2::Span;
use std::str::FromStr;
use syn::Ident;
use syn::Visibility;



pub fn parse_route_file_markdown(
	mut commands: Commands,
	query: Populated<(Entity, &RouteFile), Added<RouteFile>>,
) -> Result<()> {
	for (
		entity,
		RouteFile {
			index,
			abs_path,
			local_path,
		},
	) in query.iter().filter(|(_, file)| {
		file.abs_path.extension().map_or(false, |ext| ext == "md")
	}) {
		let mut parent = commands.entity(entity);
		let file_str = ReadFile::to_string(&abs_path)?;

		let workspace_path = abs_path.workspace_rel()?;
		let frontmatter =
			ParseMarkdown::markdown_to_frontmatter_tokens(&file_str)?;
		let rsx_str = ParseMarkdown::markdown_to_rsx_str(&file_str);
		let rust_tokens = rsx_str
			.xref()
			.xpipe(StringToWebTokens::new(workspace_path))
			.map_err(|e| {
				anyhow::anyhow!(
					"Failed to parse Markdown HTML\nPath: {}\nInput: {}\nError: {}",
					abs_path.display(),
					rsx_str,
					e.to_string()
				)
			})?
			.xpipe(WebTokensToRust::default());

		let item_fn: ItemFn = syn::parse_quote! {
			pub fn get() -> WebNode
				#rust_tokens

		};

		Ok(FuncTokens {
			mod_ident: mod_ident.clone(),
			mod_import: ModImport::Inline,
			frontmatter,
			item_fn,
			route_info: RouteInfo {
				path: RoutePath::from_file_path(&local_path)?,
				method: HttpMethod::Get,
			},
			local_path,
			abs_path,
		})
	}
	Ok(())
}


#[cfg(test)]
mod test {
	use std::ops::Deref;

	use crate::prelude::*;
	use bevy::ecs::system::RunSystemOnce;
	use bevy::prelude::*;
	use quote::ToTokens;
	use quote::quote;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let mut world = World::new();

		let group = world.spawn(FileGroup::test_site_markdown()).id();
		world.run_system_once(spawn_route_files).unwrap().unwrap();
		world
			.run_system_once(parse_route_file_markdown)
			.unwrap()
			.unwrap();
		let file = world.entity(group).get::<Children>().unwrap()[0];
		let route = world.entity(file).get::<Children>().unwrap()[0];
		let tokens = world
			.entity(route)
			.get::<FileRouteTokensSend>()
			.unwrap()
			.deref();
		// send_wrapper::SendWrapper::assert_send(&tokens);
		tokens
			.item_fn
			.to_token_stream()
			.to_string()
			.replace(" ", "")
			.xpect()
			.to_be(
				quote! {
				// pub fn get() -> WebNode {
				// 	rsx! {
				// 		<PageLayout style: cascade title="foobar">
				// 			party time!
				// 		</PageLayout>
				// 	}
				// }
				}
				.to_string()
				.replace(" ", ""),
			);
	}
}
