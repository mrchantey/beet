use beet::prelude::*;

fn main() -> Result {
	let mut app = App::new();
	app.insert_resource(CargoManifest::load()?);

	app.world_mut().spawn((
		RouteCodegenRoot::default(),
		// override default location
		CodegenFile::new(
			AbsPathBuf::new_workspace_rel(
				"examples/rsx/file_based_routes/codegen/mod.rs",
			)
			.unwrap(),
		),
		children![pages(), actions()],
	));


	Ok(())
}



fn pages() -> impl Bundle {
	(
		RouteFileCollection {
			src: AbsPathBuf::new_workspace_rel(
				"examples/rsx/file_based_routes/pages",
			)
			.unwrap(),
			..default()
		},
		CodegenFile::new(
			AbsPathBuf::new_workspace_rel(
				"examples/rsx/file_based_routes/codegen/pages.rs",
			)
			.unwrap(),
		)
		.with_pkg_name("file_based_routes"),
		// ModifyRoutePath::default()
	)
}
fn actions() -> impl Bundle {
	let actions_path = AbsPathBuf::new_workspace_rel(
		"examples/rsx/file_based_routes/codegen/actions.rs",
	)
	.unwrap();

	(
		RouteFileCollection {
			src: AbsPathBuf::new_workspace_rel(
				"examples/rsx/file_based_routes/actions",
			)
			.unwrap(),
			category: RouteCollectionCategory::Actions,
			..default()
		},
		CodegenFile::new(actions_path.clone())
			.with_pkg_name("file_based_routes"),
		// ModifyRoutePath::default()
		children![(
			CollectClientActions::default(),
			CodegenFile::new(
				AbsPathBuf::new_workspace_rel(
					"examples/rsx/file_based_routes/codegen/client_actions.rs",
				)
				.unwrap(),
			)
		)],
	)
}
