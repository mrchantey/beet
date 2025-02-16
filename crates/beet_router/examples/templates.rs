use beet_router::prelude::*;
use beet_rsx::prelude::*;

/// TODO this should be a test
/// it asserts that the process of loading tokens from macros
/// matches the process of loading tokens from the file system.
/// There are several ways this can go wrong:
/// - compile time hasher entropy differs from runtime
/// - macros discard whitespace but files do not
#[tokio::main]
async fn main() {
	let mut builder = BuildRsxTemplateMap::default();
	// builder.src = "crates/beet_router/src/test_site".into();
	builder.src = "crates/beet_router/src/test_site".into();
	builder.dst = "target/test_site/rsx-templates.ron".into();

	// 1. build
	builder.build_and_write().unwrap();

	// 2. build, parse and compare
	let tokens = builder.build_ron().unwrap();
	let map: RsxTemplateMap = ron::de::from_str(&tokens.to_string()).unwrap();

	// println!("wrote to {}\n{:#?}", builder.dst.display(), map);
	// println!("TEMPLATE_MAP::::{:#?}", map);

	let rsx =
		beet_router::test_site::routes::index::get(DefaultAppState::default());
	let root1 = map.get(&rsx.location).unwrap();
	let RsxTemplateNode::Component {
		tracker: tracker1, ..
	} = &root1.node
	else {
		panic!();
	};
	let RsxNode::Component(RsxComponent {
		tracker: tracker2, ..
	}) = &rsx.node
	else {
		panic!();
	};
	assert_eq!(tracker1, &tracker2.clone().unwrap());

	// println!("RSX:::: {:#?}", rsx);
}
