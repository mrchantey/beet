use beet_router::prelude::*;
use beet_rsx::prelude::*;
use std::collections::HashMap;

#[tokio::main]
async fn main() {
	let mut builder = BuildRsxTemplates::default();
	// builder.src = "crates/beet_router/src/test_site".into();
	builder.src = "crates/beet_router/src/test_site".into();
	builder.dst = "target/test_site/rsx-templates.ron".into();

	// 1. build
	builder.build_and_write().unwrap();

	// 2. build, parse and compare
	let tokens = builder.build_ron().unwrap();
	let map: HashMap<RsxLocation, RsxTemplateNode> =
		ron::de::from_str(&tokens.to_string()).unwrap();

	// println!("wrote to {}\n{:#?}", builder.dst.display(), map);
	// println!("TEMPLATE_MAP::::{:#?}", map);

	let rsx = beet_router::test_site::index::get(DefaultAppState::default());
	let node = map.get(&rsx.location).unwrap();
	let RsxTemplateNode::Component {
		tracker: tracker1, ..
	} = node
	else {
		panic!();
	};
	let RsxNode::Component {
		tracker: tracker2, ..
	} = rsx.node
	else {
		panic!();
	};
	assert_eq!(tracker1, &tracker2.unwrap());

	// println!("RSX:::: {:#?}", rsx);
}
