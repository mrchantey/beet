use beet_core::prelude::*;
use beet_dom::prelude::BeetRoot;
use beet_flow::prelude::*;
use beet_net::prelude::*;
use beet_rsx::prelude::*;

/// A blanket trait for both:
/// - [`IntoResponse`] types inserted verbatim
/// - [`Html<Bundle>`] types inserted as a child with [`HtmlBundle`]
trait IntoResponseBundle<M> {
	fn into_response_bundle(self) -> impl Bundle;
}

/// Used to constrain a subtype of [`Bundle`] that we can assume
/// the user would like converted to html.
pub trait IntoHtml {
	fn into_html_bundle(self) -> impl Bundle;
}
impl IntoHtml for RsxRoot {
	fn into_html_bundle(self) -> impl Bundle { children![(HtmlBundle, self)] }
}
impl<T: Bundle> IntoHtml for (RsxRoot, T) {
	fn into_html_bundle(self) -> impl Bundle { children![(HtmlBundle, self)] }
}
impl IntoHtml for BeetRoot {
	fn into_html_bundle(self) -> impl Bundle { children![(HtmlBundle, self)] }
}
impl<T: Bundle> IntoHtml for (BeetRoot, T) {
	fn into_html_bundle(self) -> impl Bundle { children![(HtmlBundle, self)] }
}
// pub struct ResultIntoBundle;
impl<T> IntoHtml for Result<T>
where
	T: IntoHtml,
{
	fn into_html_bundle(self) -> impl Bundle {
		match self {
			Ok(val) => OnSpawn::insert(val.into_html_bundle()),
			Err(err) => OnSpawn::insert(err.into_response()),
		}
	}
}


pub struct HtmlIntoResponseBundle;
impl<B> IntoResponseBundle<HtmlIntoResponseBundle> for B
where
	B: IntoHtml,
{
	fn into_response_bundle(self) -> impl Bundle { self.into_html_bundle() }
}

pub struct ResponseIntoBundle;
impl<R, M> IntoResponseBundle<(ResponseIntoBundle, M)> for R
where
	R: IntoResponse<M>,
{
	fn into_response_bundle(self) -> impl Bundle { self.into_response() }
}


struct TypeErasedResponseBundle(OnSpawn);
impl IntoResponseBundle<Self> for TypeErasedResponseBundle {
	fn into_response_bundle(self) -> impl Bundle { self.0 }
}

/// We formalize the insert step, order is critical
fn insert_and_trigger(
	world: &mut World,
	action: Entity,
	agent: Entity,
	response: impl Bundle,
	outcome: Outcome,
) {
	world.entity_mut(agent).insert(response);
	world.entity_mut(action).trigger_target(outcome);
}


/// Helper for defining methods accepting requests and returning responses.
/// These are converted to `On<GetOutcome>` observers.
/// In the case where a request cannot be found a 500 response is inserted.
pub trait IntoEndpointAction<M> {
	fn into_endpoint_action(self) -> impl Bundle;
}

/// Convenience function to convert an [`IntoEndpointAction`] into a [`BundleFunc`]
/// that can be passed to [`EndpointBuilder::with_handler`](crate::prelude::EndpointBuilder::with_handler).
///
/// Prefer using [`EndpointBuilder::with_action`](crate::prelude::EndpointBuilder::with_action)
/// which wraps this function for you.
///
/// # Example
/// ```ignore
/// EndpointBuilder::new()
///     .with_path("/foo")
///     .with_action(|req: Request| req.mirror())
/// ```
pub fn endpoint_action<M>(
	action: impl 'static + Send + Sync + Clone + IntoEndpointAction<M>,
) -> impl Bundle {
	action.into_endpoint_action()
}

/// Run the provided func, then call `into_exchange_bundle` on the output,
/// inserting it directly into the `exchange`.
fn run_and_insert<Req, Res, Func, Fut, M1, M2>(func: Func) -> impl Bundle
where
	Func: 'static + Send + Sync + Clone + FnOnce(Req, AsyncEntity) -> Fut,
	Fut: Send + Future<Output = Res>,
	Req: Send + FromRequest<M1>,
	Res: IntoResponseBundle<M2>,
{
	OnSpawn::observe(
		move |ev: On<GetOutcome>,
		      agents: AgentQuery,
		      mut commands: Commands| {
			let func = func.clone();
			let action = ev.target();
			let agent = agents.entity(action);
			assert_ne!(
				agent,
				agents.parents.root_ancestor(action),
				"agent cannot be action root, this will clobber action decendents upon scene insertion"
			);

			commands.queue(move |world: &mut World| {
				// try to take the request
				match world.entity_mut(agent).take::<Request>() {
					Some(req) => {
						world.run_async(async move |world: AsyncWorld| {
							match Req::from_request(req).await {
								Ok(req) => {
									let res =
										func(req, world.entity(action)).await;
									// insert response and trigger outcome in one world access
									// to avoid race with entity despawn after response insert
									let bundle = res.into_response_bundle();
									world
										.with_then(move |world| {
											insert_and_trigger(
												world,
												action,
												agent,
												bundle,
												Outcome::Pass,
											);
										})
										.await;
								}
								Err(response) => {
									// insert response and trigger outcome in one world access
									world
										.with_then(move |world| {
											insert_and_trigger(
												world,
												action,
												agent,
												response,
												Outcome::Fail,
											);
										})
										.await;
								}
							}
						});
					}
					None => {
						let response = bevyhow!(
							"
No Request found for endpoint, this is usually because it has already
been taken by a previous route, please check for conficting endpoints.
			"
						)
						.into_response();

						insert_and_trigger(
							world,
							action,
							agent,
							response,
							Outcome::Fail,
						);
					}
				}
			});
		},
	)
}
/// A non-func type that can be converted directly into a response bundle.
pub struct TypeIntoEndpoint;
impl<T, M> IntoEndpointAction<(TypeIntoEndpoint, M)> for T
where
	T: 'static + Send + Sync + Clone + IntoResponseBundle<M>,
{
	fn into_endpoint_action(self) -> impl Bundle {
		// skip all the async shenannigans, just insert the response
		OnSpawn::observe(
			move |ev: On<GetOutcome>,
			      agents: AgentQuery,
			      mut commands: Commands| {
				let action = ev.target();
				let agent = agents.entity(action);
				let this = self.clone();
				commands
					.entity(agent)
					// predicates like fallback depend on endpoints consuming the request
					.remove::<Request>();
				commands.queue(move |world: &mut World| {
					insert_and_trigger(
						world,
						action,
						agent,
						this.into_response_bundle(),
						Outcome::Pass,
					);
				});
			},
		)
	}
}


pub struct SystemIntoEndpoint;
impl<System, Req, Out, M1, M2, M3>
	IntoEndpointAction<(SystemIntoEndpoint, Req, Out, M1, M2, M3)> for System
where
	System: 'static + Send + Sync + Clone + IntoSystem<Req, Out, M1>,
	Req: 'static + Send + SystemInput,
	for<'a> Req::Inner<'a>: 'static + Send + Sync + FromRequest<M2>,
	Out: 'static + Send + Sync + IntoResponseBundle<M3>,
{
	fn into_endpoint_action(self) -> impl Bundle {
		run_and_insert(async move |req, action| {
			match action
				.world()
				.run_system_cached_with(self.clone(), req)
				.await
			{
				Ok(bundle) => TypeErasedResponseBundle(OnSpawn::insert(
					bundle.into_response_bundle(),
				)),
				Err(bundle) => TypeErasedResponseBundle(OnSpawn::insert(
					HttpError::from(bundle).into_response_bundle(),
				)),
			}
		})
	}
}
pub struct ActionSystemIntoEndpoint;
impl<System, Req, Out, M2, M3>
	IntoEndpointAction<(ActionSystemIntoEndpoint, Req, Out, M2, M3)> for System
where
	System: 'static + Send + Sync + Clone + FnMut(Req, AsyncEntity) -> Out,
	Req: 'static + Send + Sync + FromRequest<M2>,
	Out: 'static + Send + Sync + IntoResponseBundle<M3>,
{
	fn into_endpoint_action(self) -> impl Bundle {
		run_and_insert(async move |req: Req, action| self.clone()(req, action))
	}
}


pub struct AsyncSystemIntoEndpoint;
impl<Func, Fut, Req, Res, M1, M2>
	IntoEndpointAction<(AsyncSystemIntoEndpoint, Req, Res, M1, M2)> for Func
where
	Func: 'static + Send + Sync + Clone + FnOnce(Req, AsyncEntity) -> Fut,
	Fut: Send + Future<Output = Res>,
	Req: Send + FromRequest<M1>,
	Res: IntoResponseBundle<M2>,
{
	fn into_endpoint_action(self) -> impl Bundle { run_and_insert(self) }
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;
	use beet_rsx::prelude::*;
	use serde::Deserialize;
	use serde::Serialize;

	#[derive(Serialize, Deserialize)]
	struct Foo(u32);

	async fn assert<H, M>(
		handler: impl 'static + Send + Sync + Clone + Fn() -> H,
	) -> StatusCode
	where
		H: IntoEndpointAction<M>,
	{
		RouterPlugin::world()
			.spawn(flow_exchange(move || handler().into_endpoint_action()))
			.exchange(Request::get("/foo").with_json_body(&Foo(3)).unwrap())
			.await
			.status()
	}

	#[beet_core::test]
	async fn response() {
		assert(|| StatusCode::Ok).await.xpect_eq(StatusCode::Ok);
	}


	#[beet_core::test]
	async fn system() {
		fn my_system(_: In<Json<Foo>>) -> StatusCode { StatusCode::Ok }
		assert(|| my_system).await.xpect_eq(StatusCode::Ok);
		assert(|| |_: In<Json<Foo>>| StatusCode::Ok)
			.await
			.xpect_eq(StatusCode::Ok);
		assert(|| StatusCode::Ok).await.xpect_eq(StatusCode::Ok);
	}
	#[beet_core::test]
	async fn cx_system() {
		assert(|| |_: Json<Foo>, _: AsyncEntity| StatusCode::Ok)
			.await
			.xpect_eq(StatusCode::Ok);
		assert(|| StatusCode::Ok).await.xpect_eq(StatusCode::Ok);
	}
	#[beet_core::test]
	async fn async_system() {
		async fn my_async_system(_: Json<Foo>, _: AsyncEntity) -> StatusCode {
			StatusCode::Ok
		}
		assert(|| my_async_system).await.xpect_eq(StatusCode::Ok);
		assert(|| async |_: Json<Foo>, _: AsyncEntity| StatusCode::Ok)
			.await
			.xpect_eq(StatusCode::Ok);
	}

	#[beet_core::test]
	async fn html() {
		// just check compilation, see html_bundle for test
		let _ = assert(|| || rsx! {<div>"hello world"</div>});
		async fn foobar(
			_req: (),
			_cx: AsyncEntity,
		) -> Result<impl use<> + IntoHtml> {
			Ok(rsx! {<div>"hello world"</div>})
		}

		let _ = assert(|| foobar);
	}
}
