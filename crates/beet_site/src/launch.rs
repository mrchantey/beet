use beet::exports::syn;
use beet::prelude::*;


pub fn launch_plugin(app: &mut App) {
	let mut config = WorkspaceConfig::default();
	config
		.filter
		.include("*/crates/beet_design/src/**/*")
		.include("*/crates/beet_site/src/**/*");

	app.insert_resource(config);

	app.world_mut().spawn(collections());
}

pub fn collections() -> impl Bundle {
	(
		CodegenFile::new(
			AbsPathBuf::new_workspace_rel(
				"crates/beet_site/src/codegen/mod.rs",
			)
			.unwrap(),
		),
		children![
			route_tree(),
			pages_collection(),
			docs_collection(),
			blog_collection(),
			design_mockups_collection(),
			actions_collection()
		],
	)
}

fn route_tree() -> impl Bundle {
	(
		StaticRouteTree::default(),
		CodegenFile::new(
			AbsPathBuf::new_workspace_rel(
				"crates/beet_site/src/codegen/route_tree.rs",
			)
			.unwrap(),
		),
	)
}


fn pages_collection() -> impl Bundle {
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
fn docs_collection() -> impl Bundle {
	(
		RouteFileCollection {
			src: AbsPathBuf::new_workspace_rel("crates/beet_site/src/docs")
				.unwrap(),
			..default()
		},
		ModifyRoutePath::default().base_route("/docs"),
		MetaType::new(syn::parse_quote!(beet::prelude::ArticleMeta)),
		CodegenFile::new(
			AbsPathBuf::new_workspace_rel(
				"crates/beet_site/src/codegen/docs/mod.rs",
			)
			.unwrap(),
		),
	)
}
fn blog_collection() -> impl Bundle {
	(
		RouteFileCollection {
			src: AbsPathBuf::new_workspace_rel("crates/beet_site/src/blog")
				.unwrap(),
			..default()
		},
		ModifyRoutePath::default().base_route("/blog"),
		MetaType::new(syn::parse_quote!(beet::prelude::ArticleMeta)),
		CodegenFile::new(
			AbsPathBuf::new_workspace_rel(
				"crates/beet_site/src/codegen/blog/mod.rs",
			)
			.unwrap(),
		),
	)
}

fn design_mockups_collection() -> impl Bundle {
	(
		RouteFileCollection {
			src: AbsPathBuf::new_workspace_rel("crates/beet_design/src")
				.unwrap(),
			filter: GlobFilter::default()
				.with_include("*.mockup*")
				.with_exclude("/codegen/*"),
			..default()
		},
		ModifyRoutePath::default()
			.base_route("/docs/design")
			.replace_route(".mockup", ""),
		MetaType::new(syn::parse_quote!(crate::prelude::ArticleMeta)),
		CodegenFile::new(
			AbsPathBuf::new_workspace_rel(
				"crates/beet_design/src/codegen/mockups.rs",
			)
			.unwrap(),
		)
		.with_pkg_name("beet_design")
		.set_imports(vec![
			syn::parse_quote! {
			#[allow(unused_imports)]
			use bevy::prelude::*;},
			syn::parse_quote! {
			#[allow(unused_imports)]
			use beet_rsx::prelude::*;},
			syn::parse_quote! {
			#[allow(unused_imports)]
			use beet_dom::prelude::*;},
			syn::parse_quote! {
			#[allow(unused_imports)]
			use beet_router::prelude::*;},
			syn::parse_quote! {
			#[allow(unused_imports)]
			use crate::prelude::*;},
		]),
	)
}


fn actions_collection() -> impl Bundle {
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
			.with_import(syn::parse_quote!(
				#[allow(unused_imports)]
				use crate::prelude::*;
			))
		)],
	)
}
