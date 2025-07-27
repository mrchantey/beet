use beet::exports::syn;
use beet::prelude::*;


pub fn launch_plugin(app: &mut App) {
	app.insert_resource(CargoManifest::load().unwrap());

	app.world_mut().spawn((
		RouteCodegenRoot::default(),
		// override default location
		CodegenFile::new(
			AbsPathBuf::new_workspace_rel("src/codegen/mod.rs").unwrap(),
		),
		children![pages(), docs(), actions()],
	));

	app.set_runner(|mut app| {
		app.world_mut()
			.run_sequence_once(import_route_file_collection)
			.unwrap()
			.run_sequence_once(ParseFileSnippets)
			.unwrap()
			.run_sequence_once(RouteCodegenSequence)
			.unwrap()
			.run_sequence_once(export_route_codegen)
			.unwrap();
		AppExit::Success
	});
}




fn pages() -> impl Bundle {
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
fn docs() -> impl Bundle {
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
fn actions() -> impl Bundle {
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
