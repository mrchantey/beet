use crate::prelude::*;
use beet::exports::axum::extract::State;
use beet::exports::axum::response::Html;
use beet::prelude::*;

#[derive(Debug, Default, Clone)]
pub struct DocsLayout;

impl BundleLayerHandler for DocsLayout {
	type Extractors = State<Self::State>;
	type State = AppRouterState;
	type Output = Html<String>;
	type Meta = DocsMeta;

	fn handle_bundle_route(
		&self,
		state: Self::Extractors,
		bundle: impl Bundle,
		meta: Self::Meta,
	) -> impl Send + Sync + Future<Output = Self::Output> {
		async move {
			state.render_bundle(rsx! {
				<BaseLayout>
					<main>
						<h1>{meta.title.unwrap_or("Unnamed file".to_string())}</h1>
						<a href={routes::index()}>Home</a>
						{bundle}
					</main>
					<style>
						main {
							max-width: 800px;
							margin-left: auto;
							margin-right: auto;
						}
					</style>
				</BaseLayout>
			})
		}
	}
}
