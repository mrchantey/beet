use crate::prelude::*;
use beet::exports::axum::response::IntoResponse;
use beet::exports::axum::response::Response;
use beet::prelude::*;

#[derive(Debug, Default, Clone)]
pub struct ArticleLayout;

#[cfg(not(target_arch = "wasm32"))]
impl BundleLayerHandler for ArticleLayout {
	type Extractors = ();
	type State = ();
	type Output = Response;
	type Meta = ArticleMeta;

	fn handle_bundle_route(
		&self,
		_extractors: Self::Extractors,
		bundle: impl Bundle,
		meta: Self::Meta,
	) -> impl Send + Sync + Future<Output = Self::Output> {
		async move {
			BundleResponse::new(rsx! {
				<BeetSidebarLayout>
					<h1>{meta.title.unwrap_or("File".to_string())}</h1>
					{bundle}
				</BeetSidebarLayout>
			})
			.into_response()
		}
	}
}
