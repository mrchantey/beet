use crate::prelude::*;
use beet::prelude::*;




pub fn routes() -> impl Bundle {
	(children![
		pages_routes(),
		docs_routes(),
		blog_routes(),
		actions_routes(),
		beet_design::mockups::mockups_routes(),
		(RouteFilter::new("docs"), article_layout_middleware()),
		(RouteFilter::new("blog"), article_layout_middleware()),
		(RouteFilter::new("design"), article_layout_middleware()),
	],)
}
