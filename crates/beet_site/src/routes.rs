use crate::prelude::*;
use beet::prelude::*;



pub fn router_bundle() -> impl Bundle {
	children![
		pages_routes(),
		docs_routes(),
		blog_routes(),
		actions_routes(),
		beet_design::mockups::mockups_routes(),
		(PathFilter::new("docs"), article_layout_middleware()),
		(PathFilter::new("blog"), article_layout_middleware()),
		(PathFilter::new("design"), article_layout_middleware()),
	]
}
