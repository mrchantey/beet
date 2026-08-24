use crate::prelude::*;
use alloc::sync::Arc;
use beet_core::prelude::*;
use bevy::ecs::system::SystemState;
use core::panic::Location;

/// The one action an entity holds, keyed by its `(In, Out)` signature.
///
/// It is the sole producer of [`ActionMeta`]: inserting it inserts the meta,
/// removing it removes the meta, and a second action with a different handler
/// raises a clobber error rather than silently taking the slot. Additional
/// signatures for the same behaviour belong on an [`ActionOverload`], which
/// registers its pair in the meta and never touches the canonical fields.
#[derive(Component)]
#[component(on_insert = Action::<In, Out>::on_insert, on_remove = Action::<In, Out>::on_remove)]
pub struct Action<In: 'static, Out: 'static> {
	/// Describes this action, becoming the entity's [`ActionMeta`] on insert.
	meta: ActionMeta,
	handler: Arc<dyn 'static + Send + Sync + Fn(ActionCall<In, Out>) -> Result>,
}

impl<In: 'static, Out: 'static> core::fmt::Debug for Action<In, Out> {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("Action").field("meta", &self.meta).finish()
	}
}

impl<In: 'static, Out: 'static> Clone for Action<In, Out> {
	fn clone(&self) -> Self {
		Self {
			meta: self.meta.clone(),
			handler: Arc::clone(&self.handler),
		}
	}
}

impl<In, Out> Action<In, Out>
where
	In: 'static,
	Out: 'static,
{
	pub fn new(
		meta: ActionMeta,
		handler: impl 'static + Send + Sync + Fn(ActionCall<In, Out>) -> Result,
	) -> Self {
		Self {
			meta,
			handler: Arc::new(handler),
		}
	}

	/// Replace this action's [`ActionMeta`] with one naming the provider.
	///
	/// The factories ([`new_pure`](Self::new_pure), [`new_async`](Self::new_async),
	/// [`ContinueRun::start_running`], ...) can only describe
	/// the handler they were handed: an unnameable closure type, or a generic
	/// shared by every provider that reuses the factory. A provider requiring
	/// such an action supplies its own meta at the `#[require]` site so the meta
	/// names *it*:
	///
	/// ```text
	/// #[require(Action<Request, Response> = Router::action()
	///     .with_meta(ActionMeta::of::<Self, Request, Response>()))]
	/// ```
	///
	/// That is what makes [`assert_provider`](Self::assert_provider) work: guard
	/// B recognises the action as the one `P` provides by comparing
	/// [`handler_meta`](Self::handler_meta) to `P`, which a closure-typed meta
	/// could never match. It also gives diagnostics a real name, and lets the
	/// `#[action]` macro attach reflection data (the doc description, the
	/// input/output schemas) a closure type has none of.
	pub fn with_meta(mut self, meta: ActionMeta) -> Self {
		self.meta = meta;
		self
	}

	/// This action's descriptor, the entity's [`ActionMeta`] once inserted.
	pub fn meta(&self) -> &ActionMeta { &self.meta }

	/// The full type name of the handler, for display and debugging.
	pub fn handler_meta(&self) -> TypeMeta { *self.meta.handler() }

	/// Guard A: [`ActionMeta`] ownership. Deferred through [`Commands`] so two
	/// actions inserted in one bundle are checked against each other rather than
	/// both seeing an empty entity.
	///
	/// - no meta: insert this action's
	/// - an unset or same-handler meta: replace it, keeping any
	///   [`ActionOverload`] registrations, so a reload is idempotent
	/// - any other handler: raise a clobber error
	///
	/// Always through `insert`, never an in-place edit, so consumers observing
	/// `Insert<ActionMeta>` (route discovery) see the final meta rather than an
	/// [`unset`](ActionMeta::unset) one an overload created first.
	///
	/// `on_insert` rather than `on_add` so an overwrite is caught too: bevy runs
	/// `on_replace` then `on_insert` on overwrite and skips `on_add`.
	fn on_insert(mut world: DeferredWorld, cx: HookContext) {
		let Some(canonical) = world
			.entity(cx.entity)
			.get::<Self>()
			.map(Action::meta)
			.cloned()
		else {
			return;
		};
		let entity = cx.entity;
		let location = Location::caller();
		world.commands().queue(move |world: &mut World| {
			let Ok(entity_ref) = world.get_entity(entity) else {
				return;
			};
			let existing = entity_ref.get::<ActionMeta>().map(|meta| {
				let clobbers =
					!meta.is_unset() && *meta.handler() != *canonical.handler();
				(clobbers, meta.name(), meta.overloads().clone())
			});
			match existing {
				Some((true, existing_name, _)) => {
					let err = bevyhow!(
						"an entity holds at most one action, but {entity} already \
						 holds {existing_name}. Remove it before inserting {}.",
						canonical.name(),
					);
					world.handle_command_error_with_location::<Self>(
						err, location,
					);
				}
				Some((false, _, overloads)) => {
					world
						.entity_mut(entity)
						.insert(canonical.with_overloads(overloads));
				}
				None => {
					world.entity_mut(entity).insert(canonical);
				}
			}
		});
	}

	/// Removing the action removes its [`ActionMeta`], so a meta can never
	/// outlive the action it describes.
	fn on_remove(mut world: DeferredWorld, cx: HookContext) {
		world
			.commands()
			.entity(cx.entity)
			.try_remove::<ActionMeta>();
	}

	/// Guard B: provider integrity. Verifies the entity's `Action<In, Out>` is
	/// still the one `P` provides, catching a colocated explicit action silently
	/// taking the slot (bevy's `#[require]` yields to an explicit component), which
	/// leaves exactly one truthful action behind and so is invisible to guard A.
	///
	/// Applied in path form, since a call expression cannot name a component's own
	/// generic params:
	/// `#[component(on_add = Action::<In, Out>::assert_provider::<Self>)]`.
	#[track_caller]
	pub fn assert_provider<P: 'static>(
		mut world: DeferredWorld,
		cx: HookContext,
	) {
		let intact = world
			.entity(cx.entity)
			.get::<Self>()
			.is_some_and(|action| action.handler_meta() == TypeMeta::of::<P>());
		if intact {
			return;
		}
		let err = bevyhow!(
			"{} on {} lost its action slot: a colocated explicit Action<{}, {}> \
			 replaced the one it requires, so everything depending on it is inert. \
			 Move the explicit action (or this provider) to its own entity.",
			core::any::type_name::<P>(),
			cx.entity,
			core::any::type_name::<In>(),
			core::any::type_name::<Out>(),
		);
		world.commands().handle_command_error::<P>(err);
	}

	/// Invoke this action handler with the given [`ActionCall`].
	///
	/// # Errors
	/// Propagates any error from the handler or [`OutHandler`].
	pub fn call(&self, call: ActionCall<In, Out>) -> Result {
		(self.handler)(call)
	}

	/// Invoke this action handler, constructing the [`ActionCall`] internally.
	///
	/// # Errors
	/// Propagates any error from the handler or [`OutHandler`].
	pub fn call_with(
		&self,
		entity: Entity,
		input: In,
		commands: AsyncCommands,
		out_handler: OutHandler<Out>,
	) -> Result {
		let call = ActionCall {
			commands,
			caller: entity,
			input,
			out_handler,
		};
		self.call(call)
	}

	/// Invoke this action handler from a [`World`], constructing the [`ActionCall`] internally.
	///
	/// # Errors
	/// Propagates any error from the handler or [`OutHandler`].
	pub fn call_world(
		&self,
		entity: EntityWorldMut,
		input: In,
		out_handler: OutHandler<Out>,
	) -> Result {
		let caller = entity.id();
		let world = entity.into_world_mut();
		let mut state = SystemState::<AsyncCommands>::new(world);
		let commands = state.get_mut(world)?;
		let result = self.call_with(caller, input, commands, out_handler);
		state.apply(world);
		world.flush();
		result
	}

	/// Invoke this action handler asynchronously, constructing the [`ActionCall`] internally.
	///
	/// # Errors
	/// Propagates any error from the handler or [`OutHandler`].
	pub async fn call_async(
		&self,
		entity: AsyncEntity,
		input: In,
		out_handler: OutHandler<Out>,
	) -> Result
	where
		In: 'static + Send,
		Out: 'static + Send,
	{
		let this = self.clone();
		entity
			.with(move |entity| this.call_world(entity, input, out_handler))
			.await
			.flatten()
	}
}

/// Payload for a single action invocation, containing the caller entity,
/// input value, [`AsyncCommands`] for queuing work, and a callback
/// for delivering the output.
pub struct ActionCall<'w, 's, In, Out> {
	/// Commands for queuing ECS work or spawning async tasks.
	pub commands: AsyncCommands<'w, 's>,
	/// The entity that initiated or owns this action call.
	pub caller: Entity,
	/// The input payload for this invocation.
	pub input: In,
	/// Callback invoked with the output when the action completes.
	pub out_handler: OutHandler<Out>,
}

impl<'w, 's, In, Out> ActionCall<'w, 's, In, Out> {}

/// Delivers an action's output or error back to the caller.
///
/// Wraps a closure so that different delivery mechanisms (channels,
/// pipe chains, etc.) share a uniform interface.
pub struct OutHandler<Out = ()> {
	func: Box<
		dyn 'static
			+ Send
			+ Sync
			+ FnOnce(AsyncCommands, Result<Out>) -> Result,
	>,
}

impl<Out> Default for OutHandler<Out> {
	fn default() -> Self {
		Self {
			// by default discard the out value, propagating the error
			func: Box::new(|_, result| result.map(|_| ())),
		}
	}
}

impl<Out> OutHandler<Out> {
	/// Exit with [`AppExit::Success`] once the action call is complete,
	/// discarding the [`Out`] value.
	pub fn exit() -> Self {
		OutHandler {
			func: Box::new(|commands, result| {
				result?;
				commands.run(async |world| {
					world.write_message(AppExit::Success).await;
				});
				Ok(())
			}),
		}
	}
}

impl<Out> OutHandler<Out> {
	/// Create an [`OutHandler`] from any compatible closure.
	pub fn new<F>(func: F) -> Self
	where
		F: 'static + Send + Sync + FnOnce(AsyncCommands, Result<Out>) -> Result,
	{
		Self {
			func: Box::new(func),
		}
	}

	/// Deliver the result, consuming this handler.
	///
	/// # Errors
	/// Returns whatever error the inner callback produces.
	pub fn call(self, commands: AsyncCommands, result: Result<Out>) -> Result {
		(self.func)(commands, result)
	}

	pub fn call_world(self, world: &mut World, result: Result<Out>) -> Result {
		let mut state = SystemState::<AsyncCommands>::new(world);
		let commands = state.get_mut(world)?;
		let result = (self.func)(commands, result);
		state.apply(world);
		world.flush();
		result
	}

	pub async fn call_async(
		self,
		world: AsyncWorld,
		result: Result<Out>,
	) -> Result
	where
		Out: 'static + Send,
	{
		world
			.with(move |world| self.call_world(world, result))
			.await
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	#[should_panic = "No Action"]
	async fn missing_action_component() {
		AsyncPlugin::world()
			.spawn_empty()
			.call::<(), ()>(())
			.await
			.unwrap();
	}

	#[action(pure)]
	#[derive(Reflect)]
	fn add((a, b): (u32, u32)) -> u32 { a + b }

	/// A provider whose `#[require]` supplies the entity's action, the shape
	/// guard B protects.
	#[derive(Default, Component)]
	#[require(Action<u32, u32> = double().with_meta(ActionMeta::of::<Provider, u32, u32>()))]
	#[component(on_add = Action::<u32, u32>::assert_provider::<Self>)]
	struct Provider;

	fn double() -> Action<u32, u32> {
		Action::new_pure(|cx: ActionContext<u32>| *cx * 2)
	}
	fn increment() -> Action<u32, u32> {
		Action::new_pure(|cx: ActionContext<u32>| *cx + 1)
	}
	fn negate() -> Action<i32, i32> {
		Action::new_pure(|cx: ActionContext<i32>| -*cx)
	}

	#[beet_core::test]
	fn the_action_owns_the_meta() {
		let mut world = World::new();
		let entity = world.spawn(add.into_action()).id();
		world.flush();
		// the handler-derived meta carries no reflection data
		world
			.get::<ActionMeta>(entity)
			.unwrap()
			.type_info()
			.xpect_none();

		world.entity_mut(entity).remove::<Action<(u32, u32), u32>>();
		world.flush();
		// removing the action removes the meta it describes
		world.get::<ActionMeta>(entity).xpect_none();
	}

	#[beet_core::test]
	#[should_panic = "at most one action"]
	fn two_actions_on_one_entity_raise() {
		let mut world = World::new();
		world.spawn((add.into_action(), negate()));
		world.flush();
	}

	#[beet_core::test]
	#[should_panic = "at most one action"]
	fn overwrite_with_a_different_handler_raises() {
		let mut world = World::new();
		let entity = world.spawn(add.into_action()).id();
		world.flush();
		world.entity_mut(entity).insert(double());
		world.flush();
	}

	/// A scene reload re-inserts the same required action, which must refresh the
	/// meta in place rather than raise, keeping any overload registrations.
	#[beet_core::test]
	fn reinserting_the_same_provider_is_idempotent() {
		let mut world = World::new();
		let entity = world
			.spawn((
				Provider,
				ActionOverload::new(Action::<i32, i32>::new_pure(
					|cx: ActionContext<i32>| *cx,
				)),
			))
			.id();
		world.flush();
		world.entity_mut(entity).insert(Provider);
		world.flush();

		let meta = world.get::<ActionMeta>(entity).unwrap();
		meta.name().xpect_contains("Provider");
		meta.matches::<u32, u32>().xpect_true();
		meta.matches::<i32, i32>().xpect_true();
	}

	/// The capability-rebind shape: removing a provider's required action then
	/// inserting a replacement hands the slot over rather than raising, and the
	/// meta describes the replacement.
	#[beet_core::test]
	fn remove_then_insert_replaces_provider_action() {
		let mut world = World::new();
		let entity = world.spawn(Provider).id();
		world.flush();
		world.entity_mut(entity).remove::<Action<u32, u32>>();
		world.flush();
		// the removal really vacated the slot (a `#[require]` only materializes
		// at provider insertion, never re-inserts).
		world
			.entity(entity)
			.contains::<Action<u32, u32>>()
			.xpect_false();
		world.get::<ActionMeta>(entity).xpect_none();
		world.entity_mut(entity).insert(increment());
		world.flush();
		world
			.get::<ActionMeta>(entity)
			.unwrap()
			.name()
			.xpect_contains("increment");
	}

	/// A colocated explicit action wins over the provider's `#[require]`, leaving
	/// one truthful action behind, which only guard B can see.
	#[beet_core::test]
	#[should_panic = "lost its action slot"]
	fn provider_losing_its_slot_raises() {
		let mut world = World::new();
		world.spawn((Provider, increment()));
		world.flush();
	}

	#[beet_core::test]
	fn reflect_meta_has_type_info() {
		let meta = ActionMeta::of_reflect::<add, add>();
		meta.type_info().xpect_some();
		meta.input_info().xpect_some();
		meta.output_info().xpect_some();
	}
}
