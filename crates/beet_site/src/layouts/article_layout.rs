use crate::prelude::*;
use beet::prelude::*;


pub fn article_layout_middleware() -> EndpointBuilder {
	EndpointBuilder::layer::<(Result, _, _, _, _)>(
		|cx: In<MiddlewareContext>,
		 query: HtmlBundleQuery,
		 mut commands: Commands|
		 -> Result {
			let Some(html_bundle) = query.get(cx.exchange())? else {
				return Ok(());
			};
			// nest the current HtmlBundle under a new root
			commands.spawn((
				HtmlDocument,
				HtmlBundle,
				ChildOf(cx.exchange()),
				rsx! { <ArticleLayout>{html_bundle}</ArticleLayout> },
			));
			Ok(())
		},
	)
	.with_trailing_path()
}

#[template]
pub fn ArticleLayout(query: Query<&ArticleMeta>) -> impl Bundle {
	for _item in query.iter() {
		// blocked on immediately resolved templates
		// panic!("tadaa! {:?}", _item);
		println!("ArticleMeta: {:?}", _item);
	}

	let meta = ArticleMeta::default();
	rsx! {
		<BeetSidebarLayout>
			{meta.title.map(|title| rsx! { <h1>{title}</h1> })} <slot />
		</BeetSidebarLayout>
	}
}
