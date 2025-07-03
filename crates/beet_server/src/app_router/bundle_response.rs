use axum::response::Html;
use axum::response::IntoResponse;
use axum::response::Response;
use beet_template::prelude::*;
use bevy::prelude::*;

/// Newtype for bundle to be rendered as html string.
pub struct BundleResponse<T: 'static + Send + Sync> {
	/// The bundle content
	pub bundle: T,
}

impl<T: 'static + Send + Sync> BundleResponse<T> {
	pub fn new(bundle: T) -> Self { Self { bundle } }
}


impl<B: Bundle> IntoResponse for BundleResponse<B> {
	fn into_response(self) -> Response {
		Html(HtmlDocument::parse_bundle(self.bundle)).into_response()
	}
}
