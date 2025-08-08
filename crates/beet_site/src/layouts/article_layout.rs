use crate::prelude::*;
use beet::prelude::*;



pub fn article_layout_middleware() -> impl Bundle {
	(
		HandlerConditions::is_ssr(),
		RouteHandler::layer(|world: &mut World| {
			let entity =
				world.query_filtered_once::<Entity, With<HandlerBundle>>()[0];

			world.spawn((HtmlDocument, rsx! {
				<ArticleLayout>{entity}</ArticleLayout>
			}));
		}),
	)
}

#[template]
pub fn ArticleLayout(query: Query<&ArticleMeta>) -> impl Bundle {
	for _item in query.iter() {
		panic!("tadaa! {:?}", _item);
		// println!("ArticleMeta: {:?}", item);
	}

	let meta = ArticleMeta::default();
	rsx! {
		<BeetSidebarLayout>
			{meta.title.map(|title|rsx!{<h1>{title}</h1>})}
			<slot/>
		</BeetSidebarLayout>
	}
}
