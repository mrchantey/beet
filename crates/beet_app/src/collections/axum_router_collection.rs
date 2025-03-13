use crate::prelude::*;
use beet_server::axum::Router;


pub struct AxumRouterCollection;

impl IntoCollection<AxumRouterCollection> for Router {
	fn into_collection(self) -> impl Collection {
		move |app: &mut BeetApp| {
			app.router = std::mem::take(&mut app.router).merge(self);
		}
	}
}
