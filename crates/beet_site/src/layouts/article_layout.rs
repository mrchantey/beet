use crate::prelude::*;
use beet::prelude::*;



pub fn article_layout_middleware() -> RouteHandler {
	RouteHandler::layer(|world: &mut World| {
		let entity =
			world.query_filtered_once::<Entity, With<HandlerBundle>>()[0];
		world.spawn((HtmlDocument, rsx! {
			<ArticleLayout>{entity}</ArticleLayout>
		}));
	})
}

#[template]
pub fn ArticleLayout() -> impl Bundle {
	let meta = ArticleMeta::default();
	rsx! {
		<BeetSidebarLayout>
			<h1>{meta.title.unwrap_or("File".to_string())}</h1>
			<slot/>
		</BeetSidebarLayout>
	}
}
