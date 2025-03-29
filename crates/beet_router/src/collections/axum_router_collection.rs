use crate::prelude::*;
use beet_server::axum::Router;


pub struct AxumRouterCollection;

impl IntoCollection<AxumRouterCollection> for Router {
	fn into_collection(self) -> impl Collection {
		move |app: &mut AppRouter| {
			app.axum_router = std::mem::take(&mut app.axum_router).merge(self);
		}
	}
}
