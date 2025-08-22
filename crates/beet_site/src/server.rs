use crate::prelude::*;
use beet::prelude::*;

pub fn server_plugin(app: &mut App) {
	app.insert_resource(Router::new_bundle(routes_bundle));
}


pub fn routes_bundle() -> impl Bundle {
	children![
		pages_routes(),
		docs_routes(),
		blog_routes(),
		actions_routes(),
		beet_design::mockups::mockups_routes(),
		(PathFilter::new("docs"), article_layout_middleware()),
		(PathFilter::new("blog"), article_layout_middleware()),
	]
}
