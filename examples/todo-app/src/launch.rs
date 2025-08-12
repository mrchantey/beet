use beet::exports::syn;
use beet::prelude::*;

pub fn launch_plugin(app: &mut App) { app.world_mut().spawn(collections()); }

fn collections() -> impl Bundle {
	(
		CodegenFile::new(
			AbsPathBuf::new_workspace_rel("src/codegen/mod.rs").unwrap(),
		),
		children![
			route_tree(),
			pages_collection(),
			docs_collection(),
			actions_collection()
		],
	)
}

fn route_tree() -> impl Bundle {
	(
		StaticRouteTree::default(),
		CodegenFile::new(
			AbsPathBuf::new_workspace_rel("src/codegen/route_tree.rs").unwrap(),
		),
	)
}

fn pages_collection() -> impl Bundle {
	(
		RouteFileCollection {
			src: AbsPathBuf::new_workspace_rel("src/pages").unwrap(),
			..default()
		},
		CodegenFile::new(
			AbsPathBuf::new_workspace_rel("src/codegen/pages.rs").unwrap(),
		),
	)
}
fn docs_collection() -> impl Bundle {
	(
		RouteFileCollection {
			src: AbsPathBuf::new_workspace_rel("src/docs").unwrap(),
			..default()
		},
		ModifyRoutePath {
			base_route: Some(RoutePath::new("/docs")),
			..default()
		},
		MetaType::new(syn::parse_quote!(crate::prelude::Article)),
		CodegenFile::new(
			AbsPathBuf::new_workspace_rel("src/codegen/docs/mod.rs").unwrap(),
		),
	)
}
fn actions_collection() -> impl Bundle {
	let actions_path =
		AbsPathBuf::new_workspace_rel("src/codegen/actions.rs").unwrap();

	(
		RouteFileCollection {
			src: AbsPathBuf::new_workspace_rel("src/actions").unwrap(),
			category: RouteCollectionCategory::Actions,
			..default()
		},
		CodegenFile::new(actions_path.clone()),
		children![(
			CollectClientActions::default(),
			CodegenFile::new(
				AbsPathBuf::new_workspace_rel("src/codegen/client_actions.rs",)
					.unwrap(),
			)
		)],
	)
}
