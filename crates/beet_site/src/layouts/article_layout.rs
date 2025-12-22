use crate::prelude::*;
use beet::prelude::*;
use std::path::Path;


pub fn article_layout_middleware(path: impl AsRef<Path>) -> impl Bundle {
	MiddlewareBuilder::new::<(Result, _, _, _, _)>(
		path,
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
