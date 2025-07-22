use beet::prelude::*;

// mod codegen;

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
		children![pages(), docs(), actions()],
	));

	app.world_mut()
		.run_sequence_once(import_route_file_collection)?
		.run_sequence_once(RouteCodegenSequence)?
		.run_sequence_once(export_route_codegen)?;

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
		), // .with_pkg_name("file_based_routes"),
		   // ModifyRoutePath::default()
	)
}
fn docs() -> impl Bundle {
	(
		RouteFileCollection {
			src: AbsPathBuf::new_workspace_rel(
				"examples/rsx/file_based_routes/docs",
			)
			.unwrap(),
			..default()
		},
		ModifyRoutePath {
			base_route: Some(RoutePath::new("/docs")),
			..default()
		},
		CodegenFile::new(
			AbsPathBuf::new_workspace_rel(
				"examples/rsx/file_based_routes/codegen/docs.rs",
			)
			.unwrap(),
		),
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
