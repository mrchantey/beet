use crate::prelude::*;
use beet::prelude::*;
use std::path::Path;

/// Middleware that wraps HtmlBundle content in ArticleLayout
pub fn article_layout_middleware(path: impl AsRef<Path>) -> impl Bundle {
	let path_str = path.as_ref().to_string_lossy().to_string();
	EndpointBuilder::middleware(
		path_str,
		OnSpawn::observe(
			|ev: On<GetOutcome>,
			 agents: AgentQuery,
			 query: HtmlBundleQuery<Without<ResponseMarker>>,
			 mut commands: Commands|
			 -> Result {
				let action = ev.target();
				let agent = agents.entity(action);
				let Some(html_bundle) = query.get(agent)? else {
					commands.entity(action).trigger_target(Outcome::Pass);
					return Ok(());
				};
				// nest the current HtmlBundle under a new root
				commands.spawn((
					HtmlDocument,
					HtmlBundle,
					ChildOf(agent),
					rsx! { <ArticleLayout>{html_bundle}</ArticleLayout> },
				));
				commands.entity(action).trigger_target(Outcome::Pass);
				Ok(())
			},
		),
	)
}

#[template]
pub fn ArticleLayout(query: Query<&ArticleMeta>) -> impl Bundle {
	for _item in query.iter() {
		//TODO we should use this to generate unique page names etc
		println!("ArticleMeta: {:?}", _item);
	}

	let meta = ArticleMeta::default();
	rsx! {
		<BeetSidebarLayout>
			{meta.title.map(|title| rsx! { <h1>{title}</h1> })} <slot />
		</BeetSidebarLayout>
	}
}
