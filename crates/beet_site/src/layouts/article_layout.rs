use crate::prelude::*;
use beet::prelude::*;



pub fn article_layout_middleware() -> impl Bundle {
	(
		HandlerConditions::contains_handler_bundle(),
		RouteHandler::layer(|world: &mut World| {
			let entity = world
				.query_filtered::<Entity, With<HtmlBundle>>()
				.single(world)
				.unwrap(/*checked in handler conditions*/);

			world.spawn((HtmlDocument, rsx! { <ArticleLayout>{entity}</ArticleLayout> }));
		}),
	)
}

#[template]
pub fn ArticleLayout(query: Query<&ArticleMeta>) -> impl Bundle {
	for _item in query.iter() {
		// blocked on immediately resolved templates
		panic!("tadaa! {:?}", _item);
		// println!("ArticleMeta: {:?}", item);
	}

	let meta = ArticleMeta::default();
	rsx! {
		<BeetSidebarLayout>
			{meta.title.map(|title| rsx! { <h1>{title}</h1> })} <slot />
		</BeetSidebarLayout>
	}
}
