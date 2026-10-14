//! An extractor declares the params it reads by taking them.
//!
//! An `#[action]` taking `#[param]` arguments, a `Params<T>` or an
//! [`AnyParams`] lands its [`ParamsDeclaration`] in one of two lists on its
//! entity, chosen by its [`ParamsScope`]: [`RouteParams`] for a plain action,
//! read by the route it sits on, and [`SubtreeParams`] for middleware and
//! servers, read by every route beneath them. An extractor that is not an
//! action declares from its plugin with the initializer
//! [`declare_on`](SubtreeParams::declare_on).
use super::*;
use beet_core::prelude::*;
use bevy::reflect::TypeInfo;
use bevy::reflect::Typed;
use core::any::TypeId;

/// The params the extractors on a route entity, or on one of its steps,
/// declare: read by that route alone, see the [module](self) docs.
///
/// Derived from the extractors rather than authored, so never `Reflect`: each
/// extractor's hook rebuilds it on a scene load.
#[derive(Debug, Default, Clone, PartialEq, Deref, Component)]
pub struct RouteParams(Vec<ParamsDeclaration>);

/// The params the middleware and servers on an entity declare: read by every
/// route in the subtree it heads, see the [module](self) docs.
///
/// Derived from the extractors rather than authored, so never `Reflect`, see
/// [`RouteParams`].
#[derive(Debug, Default, Clone, PartialEq, Deref, Component)]
pub struct SubtreeParams(Vec<ParamsDeclaration>);

/// Which list a [`ParamsDeclaration`] lands in, and so which routes it
/// reaches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParamsScope {
	/// The route the extractor sits on, or runs as a step of: [`RouteParams`].
	Route,
	/// Every route at or beneath the extractor: [`SubtreeParams`].
	Subtree,
}

/// The params one extractor takes: its short type name, the [`ParamMeta`] of
/// each, and whether it takes any param ([`AnyParams`]).
#[derive(Debug, Clone, PartialEq, Get)]
pub struct ParamsDeclaration {
	/// The extractor's short type name, ie `CliServer`.
	name: SmolStr,
	/// Each param the extractor takes, in declaration order.
	metas: Vec<ParamMeta>,
	/// Whether the extractor takes any param, see [`AnyParams`].
	#[get(copy)]
	takes_any: bool,
}

impl ParamsDeclaration {
	/// The params `T` declares, read by `Extractor`: a struct, or a tuple of
	/// structs, an [`AnyParams`] member marking the extractor as taking any.
	///
	/// ## Panics
	///
	/// Panics if `T` is not a struct or tuple of structs, see [`ParamMeta::of`].
	pub fn new<Extractor: ?Sized, T: FromReflect + Typed>() -> Self {
		let mut metas = Vec::new();
		let takes_any = ParamMeta::walk(T::type_info(), &mut metas).unwrap();
		Self::named(type_ext::short_name::<Extractor>(), metas, takes_any)
	}

	/// The params an extractor built from data takes, ie an `ExchangeGroup`'s
	/// force flag, under `name`.
	pub fn named(
		name: impl Into<SmolStr>,
		metas: impl IntoIterator<Item = ParamMeta>,
		takes_any: bool,
	) -> Self {
		Self {
			name: name.into(),
			metas: metas.into_iter().collect(),
			takes_any,
		}
	}

	/// A declaration of nothing, by `Extractor`.
	pub fn empty<Extractor: ?Sized>() -> Self {
		Self::named(type_ext::short_name::<Extractor>(), [], false)
	}

	/// Whether this declares no param and does not take any.
	pub fn is_empty(&self) -> bool { self.metas.is_empty() && !self.takes_any }

	/// This declaration with `other`'s params added, keeping this name.
	pub fn with(mut self, other: Self) -> Self {
		self.metas.extend(other.metas);
		self.takes_any |= other.takes_any;
		self
	}
}

/// The identical API of both scope lists, one per `$scope`.
macro_rules! scope_list {
	($list:ident, $scope:literal) => {
		impl $list {
			/// The list holding `T`'s params read by `T`, for an extractor
			/// declared by hand, ie a closure route in a test.
			pub fn new<T: FromReflect + Typed>() -> Self {
				Self(vec![ParamsDeclaration::new::<T, T>()])
			}

			/// Add `declaration`, replacing an earlier one by the same
			/// extractor in place, so a hook re-run on a scene load lists the
			/// extractor once.
			pub fn push(&mut self, declaration: ParamsDeclaration) {
				match self
					.0
					.iter_mut()
					.find(|existing| existing.name == declaration.name)
				{
					Some(existing) => *existing = declaration,
					None => self.0.push(declaration),
				}
			}

			#[doc = concat!(
								"The `on_add` hook declaring that `Extractor` reads `T` at ",
								$scope,
								" scope, see [`push_on`](Self::push_on)."
							)]
			pub fn declare<Extractor: Component, T: FromReflect + Typed>(
				mut world: DeferredWorld,
				cx: HookContext,
			) {
				Self::queue_push(
					&mut world.commands(),
					cx.entity,
					ParamsDeclaration::new::<Extractor, T>(),
				);
			}

			#[doc = concat!(
								"The [`Initializer`] declaring that `Extractor` reads `T` at ",
								$scope,
								" scope, on every entity `Extractor` is inserted on: the \
				 declaration of an extractor that is not an `#[action]`, ie a \
				 server, made by the plugin registering it."
							)]
			pub fn declare_on<Extractor: Component, T: FromReflect + Typed>()
			-> impl Initializer {
				Self::declare_on_with::<Extractor>(ParamsDeclaration::new::<
					Extractor,
					T,
				>())
			}

			/// [`declare_on`](Self::declare_on) with an explicit
			/// `declaration`, for params read by something named apart from
			/// the component it lands with, ie the color scheme `PageClasses`
			/// reads beneath a `Layout`.
			pub fn declare_on_with<Extractor: Component>(
				declaration: ParamsDeclaration,
			) -> impl Initializer {
				move |app: &mut App| {
					app.add_observer(
						move |ev: On<Insert<Extractor>>,
						      mut commands: Commands| {
							Self::queue_push(
								&mut commands,
								ev.entity,
								declaration.clone(),
							);
						},
					);
				}
			}

			/// Push `declaration` into the list on `entity` now.
			///
			/// The list is taken off the entity and *re-inserted* rather than
			/// mutated in place (as beet_router's `MiddlewareList::on_add`
			/// does), so `On<Insert<..>>` fires for every declaration rather
			/// than only the first: the route tree rebuilds on that insert, and
			/// a step's declaration landing after its route joined the tree
			/// would otherwise go unlisted.
			fn push_on(
				entity: &mut EntityWorldMut,
				declaration: ParamsDeclaration,
			) {
				let mut list = entity.take::<Self>().unwrap_or_default();
				list.push(declaration);
				entity.insert(list);
			}

			/// Queue [`push_on`](Self::push_on) for `entity`, tolerating its
			/// despawn before the flush.
			fn queue_push(
				commands: &mut Commands,
				entity: Entity,
				declaration: ParamsDeclaration,
			) {
				commands.queue(move |world: &mut World| {
					if let Ok(mut entity) = world.get_entity_mut(entity) {
						Self::push_on(&mut entity, declaration);
					}
				});
			}
		}
	};
}

scope_list!(RouteParams, "route");
scope_list!(SubtreeParams, "subtree");

impl ParamsScope {
	/// Push `declaration` into the list this scope names on `entity` now, ie
	/// from a bundle effect.
	pub fn push(
		self,
		entity: &mut EntityWorldMut,
		declaration: ParamsDeclaration,
	) {
		match self {
			Self::Route => RouteParams::push_on(entity, declaration),
			Self::Subtree => SubtreeParams::push_on(entity, declaration),
		}
	}

	/// Queue `declaration` into the list this scope names on `entity`, ie
	/// from a component hook.
	pub fn queue_push(
		self,
		commands: &mut Commands,
		entity: Entity,
		declaration: ParamsDeclaration,
	) {
		match self {
			Self::Route => {
				RouteParams::queue_push(commands, entity, declaration)
			}
			Self::Subtree => {
				SubtreeParams::queue_push(commands, entity, declaration)
			}
		}
	}
}

/// A params group, parsed off the request through its reflected fields.
///
/// As an `#[action]` argument, `entry: Params<EntryParams>`, it is read and
/// declared before the body runs, beside any inline `#[param]` arguments. As a
/// route's input, `ActionContext<Params<T>>`, its `FromRequest` conversion
/// reads and declares it the same way.
///
/// ```
/// # use beet_core::prelude::*;
/// # use beet_net::prelude::*;
/// # use beet_action::prelude::*;
/// # // inside beet_net the macro names it `crate`
/// # use beet_net::prelude;
/// # fn main() {}
/// #[derive(Reflect)]
/// struct Verbosity {
/// 	/// Print the whole story.
/// 	verbose: bool,
/// }
///
/// #[action]
/// #[derive(Default, Clone, Component, Reflect)]
/// async fn Tell(verbosity: Params<Verbosity>) -> Result<String> {
/// 	Ok(verbosity.verbose.to_string())
/// }
/// ```
#[derive(Debug, Clone, Deref, DerefMut, TypePath)]
#[cfg_attr(feature = "serde", derive(serde::Serialize), serde(transparent))]
pub struct Params<T>(pub T);

impl<T: FromReflect + Typed> FromRequestMeta<Self> for Params<T> {
	fn from_request_meta(request: &RequestMeta) -> Result<Self, Response> {
		request
			.parse_params::<T>()
			.map(Self)
			.map_err(Response::from)
	}

	fn params() -> ParamsDeclaration { ParamsDeclaration::new::<Self, T>() }
}

#[cfg(all(feature = "serde", feature = "std"))]
impl<T: serde::Serialize> Params<T> {
	/// The params as a URL-encoded query string, the form a client sends them
	/// in.
	pub fn encode(&self) -> Result<String> {
		serde_urlencoded::to_string(&self.0)?.xok()
	}
}

/// Every param a request carries. An extractor taking it, ie an `#[action]`
/// argument `any: AnyParams`, takes any param, so a strict url space refuses
/// nothing on its route.
///
/// Reflects as an opaque value so it may sit in a declared tuple, where it
/// declares no param of its own.
#[derive(Debug, Default, Clone, PartialEq, Deref, Reflect)]
#[reflect(opaque)]
pub struct AnyParams(MultiMap<SmolStr, SmolStr>);

impl AnyParams {
	/// The request's params less every one `pattern` declares, for a route
	/// forwarding only what nothing on its path names.
	pub fn undeclared(
		&self,
		pattern: &ParamsPattern,
	) -> MultiMap<SmolStr, SmolStr> {
		let mut params = self.0.clone();
		params.retain(|key, _| !pattern.contains(key));
		params
	}

	/// Whether a reflected type is [`AnyParams`], which a declared tuple may
	/// hold.
	pub(crate) fn is(type_info: &TypeInfo) -> bool {
		type_info.type_id() == TypeId::of::<Self>()
	}
}

impl From<&RequestParts> for AnyParams {
	fn from(parts: &RequestParts) -> Self { Self(parts.params().clone()) }
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[derive(Reflect)]
	#[allow(dead_code)]
	struct VerboseParams {
		verbose: bool,
	}

	#[derive(Reflect)]
	#[allow(dead_code)]
	struct DeployParams {
		elevated: bool,
	}

	/// Stands in for middleware declaring the params it reads.
	#[derive(Component)]
	#[component(on_add = SubtreeParams::declare::<VerboseLogger, VerboseParams>)]
	struct VerboseLogger;

	/// Stands in for a second extractor on the same entity.
	#[derive(Component)]
	#[component(on_add = SubtreeParams::declare::<Deployer, DeployParams>)]
	struct Deployer;

	fn names(world: &World, entity: Entity) -> Vec<SmolStr> {
		world
			.entity(entity)
			.get::<SubtreeParams>()
			.unwrap()
			.iter()
			.map(|declaration| declaration.name().clone())
			.collect()
	}

	/// Two extractors on one entity are both listed, and a repeated hook lists
	/// its extractor once.
	#[beet_core::test]
	fn pushes_each_extractor_once() {
		let mut world = World::new();
		let entity = world.spawn((VerboseLogger, Deployer)).flush();
		names(&world, entity).xpect_eq(vec![
			SmolStr::new("VerboseLogger"),
			SmolStr::new("Deployer"),
		]);
		world.entity_mut(entity).remove::<VerboseLogger>();
		world.entity_mut(entity).insert(VerboseLogger);
		world.flush();
		names(&world, entity).len().xpect_eq(2);
	}

	/// An [`AnyParams`] member declares nothing of its own and marks the
	/// extractor as taking any param.
	#[beet_core::test]
	fn any_params_takes_any() {
		let declaration =
			ParamsDeclaration::new::<Deployer, (DeployParams, AnyParams)>();
		declaration.takes_any().xpect_true();
		declaration
			.metas()
			.iter()
			.map(ParamMeta::name)
			.collect::<Vec<_>>()
			.xpect_eq(vec!["elevated"]);
	}

	/// An extractor that is not an action declares from its plugin, landing on
	/// every entity it is inserted on.
	#[beet_core::test]
	fn declares_from_the_plugin() {
		#[derive(Component)]
		struct Server;
		let mut app = App::new();
		app.init(SubtreeParams::declare_on::<Server, VerboseParams>());
		let entity = app.world_mut().spawn(Server).flush();
		names(app.world(), entity).xpect_eq(vec![SmolStr::new("Server")]);
	}

	/// As a route input, a group parses through its reflected fields and
	/// declares them.
	#[beet_core::test]
	async fn reads_and_declares_as_an_input() {
		<Params<VerboseParams> as FromRequest<_>>::from_request(
			Request::from_cli_str("--verbose"),
		)
		.await
		.unwrap()
		.verbose
		.xpect_true();
		<Params<VerboseParams> as FromRequest<_>>::params()
			.metas()
			.iter()
			.map(ParamMeta::name)
			.collect::<Vec<_>>()
			.xpect_eq(vec!["verbose"]);
	}
}
