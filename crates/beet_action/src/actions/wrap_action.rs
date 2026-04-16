use crate::prelude::*;
use beet_core::prelude::*;

/// A handle for calling the wrapped inner action handler.
///
/// Provided to async wrapper functions so they can invoke the inner
/// handler at the point of their choosing, enabling middleware
/// patterns like input transformation, output transformation,
/// or short-circuiting.
#[derive(Get)]
pub struct Next<In: 'static, Out: 'static> {
	#[get(skip)]
	handler: Action<In, Out>,
	/// The entity that initiated this action call.
	caller: AsyncEntity,
}

impl<In, Out> Next<In, Out>
where
	In: 'static + Send + Sync,
	Out: 'static + Send + Sync,
{
	pub fn new(handler: Action<In, Out>, caller: AsyncEntity) -> Self {
		Self { handler, caller }
	}

	pub fn id(&self) -> Entity { self.caller.id() }
	pub fn world(&self) -> &AsyncWorld { self.caller.world() }

	/// Call the inner handler asynchronously.
	///
	/// Schedules the inner handler via [`AsyncWorld`] and awaits
	/// the result through a channel.
	pub async fn call(&self, input: In) -> Result<Out> {
		self.caller.call_detached(self.handler.clone(), input).await
	}
}

/// Marker for the [`IntoAction`] impl that captures async wrapper
/// closures of the form `Fn(WrapIn, Next<InnerIn, InnerOut>) -> Future<Output = Result<WrapOut>>`.
///
/// The [`Result`] is propagated as an action error, and the output type
/// is the inner `Ok` type, matching the `#[action]` macro convention.
pub struct WrapActionMarker;

impl<WrapFn, WrapIn, WrapOut, Fut, InnerIn, InnerOut>
	IntoAction<(WrapActionMarker, WrapIn, WrapOut, InnerIn, InnerOut)> for WrapFn
where
	WrapFn: 'static
		+ Send
		+ Sync
		+ Clone
		+ Fn(WrapIn, Next<InnerIn, InnerOut>) -> Fut,
	Fut: 'static + Future<Output = Result<WrapOut>>,
	WrapIn: 'static + Send + Sync,
	WrapOut: 'static + Send + Sync,
	InnerIn: 'static + Send + Sync,
	InnerOut: 'static + Send + Sync,
{
	type In = (WrapIn, Next<InnerIn, InnerOut>);
	type Out = WrapOut;

	fn into_action(self) -> Action<Self::In, Self::Out> {
		Action::new(
			TypeMeta::of::<WrapFn>(),
			move |ActionCall {
			          mut commands,
			          caller: _,
			          input: (wrap_in, next),
			          out_handler,
			      }| {
				let func = self.clone();
				commands.run_local(async move |world: AsyncWorld| -> Result {
					let result = func(wrap_in, next).await;
					out_handler.call_async(world, result).await
				});
				Ok(())
			},
		)
	}
}

/// Marker for the [`IntoAction`] impl that captures async wrapper
/// closures of the form `Fn(WrapIn, Next<InnerIn, InnerOut>) -> Future<Output = WrapOut>>`
/// where the output is NOT a [`Result`].
///
/// The output is wrapped in `Ok` automatically.
/// Disambiguated from [`WrapActionMarker`] by requiring `WrapOut: Typed`.
pub struct TypedWrapActionMarker;

impl<WrapFn, WrapIn, WrapOut, Fut, InnerIn, InnerOut>
	IntoAction<(TypedWrapActionMarker, WrapIn, WrapOut, InnerIn, InnerOut)>
	for WrapFn
where
	WrapFn: 'static
		+ Send
		+ Sync
		+ Clone
		+ Fn(WrapIn, Next<InnerIn, InnerOut>) -> Fut,
	Fut: 'static + Future<Output = WrapOut>,
	WrapIn: 'static + Send + Sync,
	WrapOut: 'static + Send + Sync + bevy::reflect::Typed,
	InnerIn: 'static + Send + Sync,
	InnerOut: 'static + Send + Sync,
{
	type In = (WrapIn, Next<InnerIn, InnerOut>);
	type Out = WrapOut;

	fn into_action(self) -> Action<Self::In, Self::Out> {
		Action::new(
			TypeMeta::of::<WrapFn>(),
			move |ActionCall {
			          mut commands,
			          caller: _,
			          input: (wrap_in, next),
			          out_handler,
			      }| {
				let func = self.clone();
				commands.run_local(async move |world: AsyncWorld| -> Result {
					out_handler
						.call_async(world, func(wrap_in, next).await.xok())
						.await
				});
				Ok(())
			},
		)
	}
}

/// Allows wrapping an action handler with middleware-style logic.
///
/// The wrapper function receives the outer input and a [`Next`]
/// handle, returning the outer output. The inner handler is
/// called via [`Next::call`] at the wrapper's discretion.
///
/// This is blanket-implemented for any [`IntoAction`] whose
/// input type is `(WrapIn, Next<InnerIn, InnerOut>)`.
pub trait IntoWrapAction<M, WrapIn, WrapOut, InnerIn, InnerOut>: Sized {
	/// Wrap an inner handler, producing a combined [`Action`].
	///
	/// The resulting handler accepts `WrapIn` and produces `WrapOut`,
	/// with the wrapper controlling when and how the inner handler
	/// (accepting `InnerIn`/`InnerOut`) is invoked via [`Next`].
	fn wrap<Inner, InnerM>(self, inner: Inner) -> Action<WrapIn, WrapOut>
	where
		Inner: 'static + IntoAction<InnerM, In = InnerIn, Out = InnerOut>,
		InnerIn: 'static + Send + Sync,
		InnerOut: 'static + Send + Sync;
}

/// Blanket impl: any [`IntoAction`] with `In = (WrapIn, Next<InnerIn, InnerOut>)`
/// automatically becomes wrappable.
// here OuterIn/OuterOut are the types for the actual action
impl<T, M, OuterIn, OuterOut, InnerIn, InnerOut>
	IntoWrapAction<M, OuterIn, OuterOut, InnerIn, InnerOut> for T
where
	T: 'static
		+ IntoAction<M, In = (OuterIn, Next<InnerIn, InnerOut>), Out = OuterOut>,
	OuterIn: 'static + Send + Sync,
	OuterOut: 'static + Send + Sync,
	InnerIn: 'static + Send + Sync,
	InnerOut: 'static + Send + Sync,
{
	fn wrap<Inner, InnerM>(self, inner: Inner) -> Action<OuterIn, OuterOut>
	where
		Inner: 'static + IntoAction<InnerM, In = InnerIn, Out = InnerOut>,
	{
		let inner_handler = inner.into_action();
		let outer_handler = self.into_action();

		Action::new(
			TypeMeta::of::<(T, Inner)>(),
			move |ActionCall {
			          commands,
			          caller,
			          input,
			          out_handler,
			      }| {
				let next = Next {
					handler: inner_handler.clone(),
					caller: commands.world().entity(caller),
				};

				outer_handler.call(ActionCall {
					commands,
					caller,
					input: (input, next),
					out_handler,
				})
			},
		)
	}
}

#[cfg(test)]
mod test {
	use std::marker::PhantomData;
	use std::str::FromStr;

	use crate::prelude::*;
	use beet_core::prelude::*;
	use bevy::ecs::entity::EntityHashMap;

	#[action(pure)]
	fn add((a, b): (i32, i32)) -> i32 { a + b }
	#[action(pure)]
	fn double(val: i32) -> i32 { val * 2 }
	#[action(pure)]
	fn negate(val: i32) -> i32 { -val }

	// -- serde roundtrip helpers ------------------------------------------

	/// Trait for types whose action handler can be constructed statically,
	/// allowing a wrapper to build the inner action at `#[require]` time.
	trait InnerTestAction: 'static + Send + Sync {
		fn inner_action() -> Action<i32, i32>;
	}

	/// Inner action defined with `#[action]`, used as the wrapped target.
	#[action(pure)]
	#[derive(Debug, Reflect)]
	fn Doubler(val: i32) -> i32 { val * 2 }

	impl InnerTestAction for Doubler {
		fn inner_action() -> Action<i32, i32> { Doubler.into_action() }
	}

	/// Wrapper function defined with `#[action]`, provides the wrapping logic.
	#[action]
	async fn AddOneWrap(
		cx: ActionContext<(i32, Next<i32, i32>)>,
	) -> Result<i32> {
		let inner_out = cx.1.call(cx.0).await?;
		Ok(inner_out + 1)
	}

	/// Serializable wrapper component, generic over the inner action `T`.
	/// Uses `#[require]` to provide the wrapped action at bundle-resolution
	/// time, avoiding timing issues during scene deserialization.
	#[derive(Debug, Clone, Component, Reflect)]
	#[reflect(Component, Default)]
	#[require(Action<i32, i32> = AddOneWrapper::<T>::make_action())]
	struct AddOneWrapper<T: InnerTestAction = Doubler>(
		#[reflect(ignore)] PhantomData<fn() -> T>,
	);

	impl<T: InnerTestAction> Default for AddOneWrapper<T> {
		fn default() -> Self { Self(PhantomData) }
	}

	impl<T: InnerTestAction> AddOneWrapper<T> {
		fn make_action() -> Action<i32, i32> {
			let inner = T::inner_action();
			Action::new_async(move |cx: ActionContext<i32>| {
				let inner = inner.clone();
				async move {
					let inner_out = cx.caller.call_detached(inner, *cx).await?;
					Ok(inner_out + 1)
				}
			})
		}
	}

	/// Example middleware accepting and returning an opaque type
	async fn my_middleware<In, Out>(
		input: String,
		next: Next<In, Out>,
	) -> Result<String>
	where
		In: 'static + Send + Sync + FromStr,
		Out: 'static + Send + Sync + ToString,
		In::Err: std::fmt::Debug,
	{
		let parsed: In = input.parse().map_err(|err| bevyhow!("{err:?}"))?;
		let output = next.call(parsed).await?;
		Ok(format!("output: {}", output.to_string()))
	}

	#[beet_core::test]
	async fn transforms_input_and_output() {
		AsyncPlugin::world()
			.spawn(my_middleware.wrap(double))
			.call::<String, String>("21".into())
			.await
			.unwrap()
			.xpect_eq("output: 42".to_string());
	}

	#[beet_core::test]
	async fn passthrough() {
		AsyncPlugin::world()
			.spawn(
				(async |input: i32, next: Next<i32, i32>| -> Result<i32> {
					next.call(input).await
				})
				.wrap(negate),
			)
			.call::<i32, i32>(5)
			.await
			.unwrap()
			.xpect_eq(-5);
	}

	#[beet_core::test]
	async fn short_circuit() {
		AsyncPlugin::world()
			.spawn(
				(async |input: i32, _next: Next<i32, i32>| -> i32 {
					// never calls inner
					input * 100
				})
				.wrap(negate),
			)
			.call::<i32, i32>(3)
			.await
			.unwrap()
			.xpect_eq(300);
	}

	#[beet_core::test]
	async fn with_tuple_inner() {
		AsyncPlugin::world()
			.spawn(
				(async |input: (i32, i32),
				        next: Next<(i32, i32), i32>|
				       -> Result<i32> {
					let inner_out = next.call(input).await?;
					Ok(inner_out + 1)
				})
				.wrap(add),
			)
			.call::<(i32, i32), i32>((3, 4))
			.await
			.unwrap()
			.xpect_eq(8);
	}

	#[beet_core::test]
	async fn called_multiple_times() {
		let mut world = AsyncPlugin::world();
		let entity = world
			.spawn(
				(async |input: i32, next: Next<i32, i32>| -> Result<i32> {
					next.call(input).await
				})
				.wrap(double),
			)
			.id();

		world
			.entity_mut(entity)
			.call::<i32, i32>(5)
			.await
			.unwrap()
			.xpect_eq(10);

		world
			.entity_mut(entity)
			.call::<i32, i32>(7)
			.await
			.unwrap()
			.xpect_eq(14);
	}

	#[beet_core::test]
	async fn modifies_inner_input_and_output() {
		AsyncPlugin::world()
			.spawn(
				(async |input: i32, next: Next<i32, i32>| -> Result<i32> {
					let inner_out = next.call(input * 10).await?;
					Ok(inner_out + 1)
				})
				.wrap(negate),
			)
			.call::<i32, i32>(3)
			.await
			.unwrap()
			.xpect_eq(-29);
	}

	// -- serde roundtrip tests --------------------------------------------

	fn scene_app() -> App {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, AsyncPlugin));
		app.register_type::<AddOneWrapper<Doubler>>();
		app.register_type::<i32>();
		app.init();
		app.update();
		app
	}

	#[beet_core::test]
	async fn wrapper_works_directly() {
		let mut app = scene_app();
		// Only AddOneWrapper is needed — its #[require] provides the
		// wrapped action, constructing the inner Doubler action statically.
		let entity = app
			.world_mut()
			.spawn(AddOneWrapper::<Doubler>::default())
			.id();
		app.update();

		// double(5) + 1 = 11
		app.world_mut()
			.entity_mut(entity)
			.call::<i32, i32>(5)
			.await
			.unwrap()
			.xpect_eq(11);
	}

	#[test]
	fn wrapper_scene_roundtrip() {
		let mut app = scene_app();

		let entity = app
			.world_mut()
			.spawn(AddOneWrapper::<Doubler>::default())
			.id();
		app.update();

		// The entity should have the wrapper component and a ActionMeta.
		app.world()
			.entity(entity)
			.get::<AddOneWrapper<Doubler>>()
			.xpect_some();
		app.world().entity(entity).get::<ActionMeta>().xpect_some();

		// Serialize
		let scene_bytes = SceneSaver::new(app.world_mut())
			.with_entities([entity])
			.save(MediaType::Ron)
			.unwrap();
		scene_bytes
			.as_utf8()
			.unwrap()
			.xref()
			.xpect_contains("AddOneWrapper");

		// Despawn original
		app.world_mut().entity_mut(entity).despawn();
		app.update();

		// Deserialize
		let mut entity_map = EntityHashMap::default();
		SceneLoader::new(app.world_mut())
			.with_entity_map(&mut entity_map)
			.load(&scene_bytes)
			.unwrap();
		app.update();

		// The loaded entity should have the wrapper and a ActionMeta
		// (Action itself isn't serializable, but #[require] re-creates it)
		let loaded = *entity_map.values().next().unwrap();
		app.world()
			.entity(loaded)
			.get::<AddOneWrapper<Doubler>>()
			.xpect_some();
		app.world().entity(loaded).get::<ActionMeta>().xpect_some();
	}

	#[beet_core::test]
	async fn wrapper_works_after_roundtrip() {
		let mut app = scene_app();

		let entity = app
			.world_mut()
			.spawn(AddOneWrapper::<Doubler>::default())
			.id();
		app.update();

		// Serialize then despawn
		let scene_bytes = SceneSaver::new(app.world_mut())
			.with_entities([entity])
			.save(MediaType::Ron)
			.unwrap();
		app.world_mut().entity_mut(entity).despawn();
		app.update();

		// Deserialize
		let mut entity_map = EntityHashMap::default();
		SceneLoader::new(app.world_mut())
			.with_entity_map(&mut entity_map)
			.load(&scene_bytes)
			.unwrap();
		app.update();

		// Call the wrapped action on the loaded entity: double(5) + 1 = 11
		let loaded = *entity_map.values().next().unwrap();
		app.world_mut()
			.entity_mut(loaded)
			.call::<i32, i32>(5)
			.await
			.unwrap()
			.xpect_eq(11);
	}
}
