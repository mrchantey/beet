use axum::Router;


pub trait RouterPlugin {
	type State;
	type Meta;
	fn build(self, router: Router<Self::State>) -> Router<Self::State>;

	fn add_route<M>(
		&self,
		router: Router<Self::State>,
		handler: impl IntoBeetRoute<M, State = Self::State>,
	) -> Router<Self::State> {
		handler.into_beet_route(router)
	}
}

pub trait IntoBeetRoute<M> {
	type State;
	fn into_beet_route(
		self,
		router: Router<Self::State>,
	) -> Router<Self::State>;
}
