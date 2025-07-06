use crate::prelude::*;
#[cfg(not(target_arch = "wasm32"))]
use beet::exports::axum::extract::State;
#[cfg(not(target_arch = "wasm32"))]
use beet::exports::axum::response::Html;
use beet::prelude::*;

#[derive(Debug, Default, Clone)]
pub struct ArticleLayout;

#[cfg(not(target_arch = "wasm32"))]
impl BundleLayerHandler for ArticleLayout {
	type Extractors = State<Self::State>;
	type State = AppRouterState;
	type Output = Html<String>;
	type Meta = ArticleMeta;

	fn handle_bundle_route(
		&self,
		state: Self::Extractors,
		bundle: impl Bundle,
		meta: Self::Meta,
	) -> impl Send + Sync + Future<Output = Self::Output> {
		async move {
			state.render_bundle(rsx! {
				<BeetSidebarLayout>
					<h1>{meta.title.unwrap_or("File".to_string())}</h1>
					{bundle}
				</BeetSidebarLayout>
			})
		}
	}
}
