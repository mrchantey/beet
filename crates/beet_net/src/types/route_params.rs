//! A reader declares the params it reads by taking them.
//!
//! An `#[action]` taking `#[param]` arguments or a `Params<T>` lands its
//! [`ParamsDeclaration`] in one of two lists on its entity, chosen by its
//! reach: [`RouteParams`] for a plain action, read by the route it sits on, and
//! [`MiddlewareParams`] for middleware and servers, read by every route
//! beneath them. A reader that is not an action declares from its plugin with
//! [`DeclareParamsAppExt::declare_params`].
use super::*;
use beet_core::prelude::*;
use bevy::reflect::TypeInfo;
use bevy::reflect::Typed;
use core::any::TypeId;

/// The params the readers on a route entity, or on one of its steps, declare:
/// read by that route alone, see the [module](self) docs.
///
/// Derived from the readers rather than authored, so never `Reflect`: each
/// reader's hook rebuilds it on a scene load, as [`MiddlewareParams`] is.
#[derive(Debug, Default, Clone, PartialEq, Deref, Component)]
pub struct RouteParams(Vec<ParamsDeclaration>);

/// The params the middleware and servers on an entity declare: read by every
/// route at or beneath it, see the [module](self) docs.
///
/// Derived from the readers rather than authored, so never `Reflect`, see
/// [`RouteParams`].
#[derive(Debug, Default, Clone, PartialEq, Deref, Component)]
pub struct MiddlewareParams(Vec<ParamsDeclaration>);

/// Which list a [`ParamsDeclaration`] lands in, and so which routes it
/// reaches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Reach {
	/// The route the reader sits on, or runs as a step of: [`RouteParams`].
	Route,
	/// Every route at or beneath the reader: [`MiddlewareParams`].
	Middleware,
}

/// The params one reader takes: its short type name, the [`ParamMeta`] of
/// each, and whether it takes the rest ([`Rest`]).
#[derive(Debug, Clone, PartialEq, Get)]
pub struct ParamsDeclaration {
	/// The reader's short type name, ie `CliServer`.
	reader: SmolStr,
	/// Each param the reader takes, in declaration order.
	metas: Vec<ParamMeta>,
	/// Whether the reader takes every param, see [`Rest`].
	#[get(copy)]
	rest: bool,
}

impl ParamsDeclaration {
	/// The params `T` declares, read by `Reader`: a struct, or a tuple of
	/// structs, a [`Rest`] member marking the reader a rest taker.
	///
	/// ## Panics
	///
	/// Panics if `T` is not a struct or tuple of structs, see [`ParamMeta::of`].
	pub fn new<Reader: ?Sized, T: FromReflect + Typed>() -> Self {
		let mut metas = Vec::new();
		let rest = ParamMeta::walk(T::type_info(), &mut metas).unwrap();
		Self::named(type_ext::short_name::<Reader>(), metas, rest)
	}

	/// The params a reader built from data takes, ie an `ExchangeGroup`'s
	/// force flag, named by `reader`.
	pub fn named(
		reader: impl Into<SmolStr>,
		metas: impl IntoIterator<Item = ParamMeta>,
		rest: bool,
	) -> Self {
		Self {
			reader: reader.into(),
			metas: metas.into_iter().collect(),
			rest,
		}
	}

	/// A declaration of nothing, read by `Reader`.
	pub fn empty<Reader: ?Sized>() -> Self {
		Self::named(type_ext::short_name::<Reader>(), [], false)
	}

	/// Whether this declares no param and does not take the rest.
	pub fn is_empty(&self) -> bool { self.metas.is_empty() && !self.rest }

	/// This declaration with `other`'s params added, keeping this reader.
	pub fn with(mut self, other: Self) -> Self {
		self.metas.extend(other.metas);
		self.rest |= other.rest;
		self
	}
}

/// The identical API of both reach lists, one per `$reach`.
macro_rules! reach_list {
	($list:ident, $reach:literal) => {
		impl $list {
			/// The list holding `T`'s params read by `T`, for a reader declared
			/// by hand, ie a closure route in a test.
			pub fn new<T: FromReflect + Typed>() -> Self {
				Self(vec![ParamsDeclaration::new::<T, T>()])
			}

			/// Add `declaration`, replacing an earlier one by the same reader in
			/// place, so a hook re-run on a scene load lists the reader once.
			pub fn push(&mut self, declaration: ParamsDeclaration) {
				match self
					.0
					.iter_mut()
					.find(|existing| existing.reader == declaration.reader)
				{
					Some(existing) => *existing = declaration,
					None => self.0.push(declaration),
				}
			}

			#[doc = concat!(
								"The `on_add` hook declaring that `Reader` reads `T` at ",
								$reach,
								" reach, see [`push_on`](Self::push_on)."
							)]
			pub fn declare<Reader: Component, T: FromReflect + Typed>(
				mut world: DeferredWorld,
				cx: HookContext,
			) {
				Self::queue_push(
					&mut world.commands(),
					cx.entity,
					ParamsDeclaration::new::<Reader, T>(),
				);
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

reach_list!(RouteParams, "route");
reach_list!(MiddlewareParams, "middleware");

impl Reach {
	/// Push `declaration` into the list this reach names on `entity` now, ie
	/// from a bundle effect.
	pub fn push(
		self,
		entity: &mut EntityWorldMut,
		declaration: ParamsDeclaration,
	) {
		match self {
			Self::Route => RouteParams::push_on(entity, declaration),
			Self::Middleware => MiddlewareParams::push_on(entity, declaration),
		}
	}

	/// Queue `declaration` into the list this reach names on `entity`, ie
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
			Self::Middleware => {
				MiddlewareParams::queue_push(commands, entity, declaration)
			}
		}
	}
}

/// A params group an `#[action]` takes, parsed off the request before the body
/// runs: `entry: Params<EntryParams>` reads and declares every field of
/// `EntryParams`, beside any inline `#[param]` arguments.
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
/// async fn Tell(
/// 	verbosity: Params<Verbosity>,
/// 	_cx: ActionContext<RequestParts>,
/// ) -> Result<String> {
/// 	Ok(verbosity.verbose.to_string())
/// }
/// ```
#[derive(Debug, Clone, Deref, DerefMut)]
pub struct Params<T>(pub T);

/// Every param a request carries, the type a route taking the rest reads as
/// `rest: Params<Rest>`: a strict url space refuses nothing on that route.
///
/// Reflects as an opaque value so it may sit in a declared tuple, where it
/// declares no param of its own.
#[derive(Debug, Default, Clone, PartialEq, Deref, Reflect)]
#[reflect(opaque)]
pub struct Rest(MultiMap<SmolStr, SmolStr>);

impl Rest {
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

	/// Whether a reflected type is [`Rest`], the rest taker a declared tuple
	/// may hold.
	pub(crate) fn is(type_info: &TypeInfo) -> bool {
		type_info.type_id() == TypeId::of::<Self>()
	}
}

impl From<&RequestParts> for Rest {
	fn from(parts: &RequestParts) -> Self { Self(parts.params().clone()) }
}

/// Declares the params a reader that is not an `#[action]` takes, ie a server
/// or a `Layout`, from the plugin registering it.
#[extend::ext(name=DeclareParamsAppExt)]
pub impl App {
	/// Declare that `Reader` takes `declaration` at `reach`, landing it on
	/// every entity `Reader` is inserted on.
	fn declare_params<Reader: Component>(
		&mut self,
		reach: Reach,
		declaration: ParamsDeclaration,
	) -> &mut Self {
		self.add_observer(
			move |ev: On<Insert<Reader>>, mut commands: Commands| {
				reach.queue_push(&mut commands, ev.entity, declaration.clone());
			},
		)
	}
}

/// The request an action input carries, for the `#[action]` macro to read
/// params through whatever shape the input takes.
pub trait RequestPartsRef {
	/// The request parts.
	fn request_parts(&self) -> &RequestParts;
}

impl RequestPartsRef for RequestParts {
	fn request_parts(&self) -> &RequestParts { self }
}

impl RequestPartsRef for Request {
	fn request_parts(&self) -> &RequestParts { Request::request_parts(self) }
}

#[cfg(feature = "action")]
impl RequestPartsRef
	for (Request, beet_action::prelude::Next<Request, Response>)
{
	fn request_parts(&self) -> &RequestParts { self.0.request_parts() }
}

#[cfg(feature = "action")]
impl RequestPartsRef
	for (
		RequestParts,
		beet_action::prelude::Next<RequestParts, Response>,
	)
{
	fn request_parts(&self) -> &RequestParts { &self.0 }
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
	#[component(on_add = MiddlewareParams::declare::<VerboseLogger, VerboseParams>)]
	struct VerboseLogger;

	/// Stands in for a second reader on the same entity.
	#[derive(Component)]
	#[component(on_add = MiddlewareParams::declare::<Deployer, DeployParams>)]
	struct Deployer;

	fn readers(world: &World, entity: Entity) -> Vec<SmolStr> {
		world
			.entity(entity)
			.get::<MiddlewareParams>()
			.unwrap()
			.iter()
			.map(|declaration| declaration.reader().clone())
			.collect()
	}

	/// Two readers on one entity are both listed, and a repeated hook lists
	/// its reader once.
	#[beet_core::test]
	fn pushes_each_reader_once() {
		let mut world = World::new();
		let entity = world.spawn((VerboseLogger, Deployer)).flush();
		readers(&world, entity).xpect_eq(vec![
			SmolStr::new("VerboseLogger"),
			SmolStr::new("Deployer"),
		]);
		world.entity_mut(entity).remove::<VerboseLogger>();
		world.entity_mut(entity).insert(VerboseLogger);
		world.flush();
		readers(&world, entity).len().xpect_eq(2);
	}

	/// A [`Rest`] member declares nothing of its own and marks the reader a
	/// rest taker.
	#[beet_core::test]
	fn rest_marks_the_rest_taker() {
		let declaration =
			ParamsDeclaration::new::<Deployer, (DeployParams, Rest)>();
		declaration.rest().xpect_true();
		declaration
			.metas()
			.iter()
			.map(ParamMeta::name)
			.collect::<Vec<_>>()
			.xpect_eq(vec!["elevated"]);
	}

	/// A reader that is not an action declares from its plugin, landing on
	/// every entity it is inserted on.
	#[beet_core::test]
	fn declares_from_the_plugin() {
		#[derive(Component)]
		struct Server;
		let mut app = App::new();
		app.declare_params::<Server>(
			Reach::Middleware,
			ParamsDeclaration::new::<Server, VerboseParams>(),
		);
		let entity = app.world_mut().spawn(Server).flush();
		readers(app.world(), entity).xpect_eq(vec![SmolStr::new("Server")]);
	}
}
