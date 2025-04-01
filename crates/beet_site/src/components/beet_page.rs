use crate::route_tree;
use beet::design::exports::*;
use beet::prelude::*;


#[derive(Node)]
pub struct BeetPage {}


fn beet_page(_: BeetPage) -> RsxNode {
	set_context(Brand {
		title: "Beet".into(),
		description: "A Rust web framework".into(),
		site_url: "https://beetrsx.dev".into(),
	});

	let brand = get_context::<Brand>();

	let routes = route_tree::collect_static_route_tree();

	let nav_items = move || {
		routes
			.flatten()
			.iter()
			.map(|route| {
				let route_str = route.to_string_lossy().to_string();
				rsx! {<a href={route_str.clone()}>{route_str}</a>}
			})
			.collect::<Vec<_>>()
	};
	let theme = ThemeBuilder::with_source(Argb::new(255, 0, 255, 127)).build();

	rsx! {
		<ContentLayout>
		<DesignSystem theme=theme/>
		// <meta slot="head" name="foo" content="bar"/>
		<h1>{brand.title}</h1>
			<nav>
				{nav_items}
			</nav>
				<slot/>
				<style>
					h1{
						padding-top: 20px;
					}
					nav{
						display: flex;
						flex-direction: column;
					}
				</style>
				<style scope:global>
					body{
						margin:0;
						background-color: black;
						color:white;
					}
				</style>
		</ContentLayout>
	}
}
