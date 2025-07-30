use beet::exports::syn;
use beet::prelude::*;


pub fn pages_collection() -> impl Bundle {
	(
		RouteFileCollection {
			src: AbsPathBuf::new_workspace_rel("crates/beet_site/src/pages")
				.unwrap(),
			..default()
		},
		CodegenFile::new(
			AbsPathBuf::new_workspace_rel(
				"crates/beet_site/src/codegen/pages.rs",
			)
			.unwrap(),
		),
	)
}
pub fn docs_collection() -> impl Bundle {
	(
		RouteFileCollection {
			src: AbsPathBuf::new_workspace_rel("crates/beet_site/src/docs")
				.unwrap(),
			..default()
		},
		ModifyRoutePath {
			base_route: Some(RoutePath::new("/docs")),
			..default()
		},
		MetaType::new(syn::parse_quote!(crate::prelude::Article)),
		CodegenFile::new(
			AbsPathBuf::new_workspace_rel(
				"crates/beet_site/src/codegen/docs/mod.rs",
			)
			.unwrap(),
		),
	)
}
pub fn actions_collection() -> impl Bundle {
	let actions_path = AbsPathBuf::new_workspace_rel(
		"crates/beet_site/src/codegen/actions.rs",
	)
	.unwrap();

	(
		RouteFileCollection {
			src: AbsPathBuf::new_workspace_rel("crates/beet_site/src/actions")
				.unwrap(),
			category: RouteCollectionCategory::Actions,
			..default()
		},
		CodegenFile::new(actions_path.clone()),
		children![(
			CollectClientActions::default(),
			CodegenFile::new(
				AbsPathBuf::new_workspace_rel(
					"crates/beet_site/src/codegen/client_actions.rs",
				)
				.unwrap(),
			)
		)],
	)
}
