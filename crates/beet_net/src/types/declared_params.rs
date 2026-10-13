//! What a request may carry: every param declared along the path that
//! dispatched it, and the check a strict server makes against them.
use super::*;
use beet_core::prelude::*;
use bevy::ecs::component::Components;
use bevy::reflect::CreateTypeData;

/// A component that reads params off the requests dispatched at or beneath its
/// entity, declaring them so [`DeclaredParams`] recognises them.
///
/// The per-type counterpart of a route's [`ParamsPartial`]: an entity holds one
/// partial, but any number of middleware and servers, so each declares the
/// params it reads on its own type, as reflect type data that
/// [`DeclaredParamsQuery`] finds through the [`AppTypeRegistry`]:
///
/// ```
/// # use beet_core::prelude::*;
/// # use beet_net::prelude::*;
/// #[derive(Reflect)]
/// struct VerboseParams {
/// 	/// Log every step.
/// 	verbose: bool,
/// }
///
/// #[derive(Default, Component, Reflect)]
/// #[reflect(Component, DeclaresParams)]
/// struct VerboseLogger;
///
/// impl DeclaresParams for VerboseLogger {
/// 	fn declared_params() -> ParamsPartial {
/// 		ParamsPartial::new::<VerboseParams>()
/// 	}
/// }
/// ```
pub trait DeclaresParams: Component {
	/// The params this component reads off a request.
	fn declared_params() -> ParamsPartial;
}

/// The [`DeclaresParams`] type data, registered by `#[reflect(DeclaresParams)]`.
#[derive(Clone)]
pub struct ReflectDeclaresParams(fn() -> ParamsPartial);

impl ReflectDeclaresParams {
	/// The params the reflected type declares.
	pub fn declared_params(&self) -> ParamsPartial { (self.0)() }
}

impl<T: DeclaresParams> CreateTypeData<T> for ReflectDeclaresParams {
	fn create_type_data(_input: ()) -> Self { Self(T::declared_params) }
}

/// Every param declared along the path a request was dispatched through, a
/// union of keys resolved by [`DeclaredParamsQuery`], each kept beside its
/// declarer so `--help` lists exactly what a strict server accepts.
///
/// Unlike a [`ParamsPattern`] it never checks two declarations of one key for
/// conflict: the launch's `--features` and `build-wasm`'s are two readers of
/// the same param, not a contradiction.
#[derive(Debug, Default, Clone)]
pub struct DeclaredParams {
	/// Each declaration in the order added.
	declarations: Vec<ParamsDeclaration>,
	/// Whether a declaration forwards every param it does not name to a program
	/// it runs, see [`ParamsPartial::forwarding`].
	forwards: bool,
}

/// The params one declarer along a dispatch path reads.
#[derive(Debug, Clone)]
pub struct ParamsDeclaration {
	/// Who reads them.
	pub declarer: ParamsDeclarer,
	/// The params read.
	pub params: Vec<ParamMeta>,
}

/// Who declares a [`ParamsDeclaration`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamsDeclarer {
	/// The route itself: a [`ParamsPartial`] on it, an ancestor or one of its
	/// steps, the params its [`ParamsPattern`] collects.
	Route,
	/// A [`DeclaresParams`] component on the route or an ancestor, ie
	/// middleware, a server or the launch it serves.
	Component {
		/// The entity the component sits on.
		entity: Entity,
		/// The component's short type path, ie `CliServer`.
		name: &'static str,
	},
}

impl DeclaredParams {
	/// Add the params `partial` declares for the route.
	pub fn extend(&mut self, partial: &ParamsPartial) {
		self.extend_from(ParamsDeclarer::Route, partial);
	}

	/// Add the params `partial` declares, read by `declarer`.
	fn extend_from(
		&mut self,
		declarer: ParamsDeclarer,
		partial: &ParamsPartial,
	) {
		self.declarations.push(ParamsDeclaration {
			declarer,
			params: partial.iter().cloned().collect(),
		});
		self.forwards |= partial.forwards();
	}

	/// Add the params `metas` declare for the route, ie its [`ParamsPattern`].
	pub fn extend_metas<'a>(
		&mut self,
		metas: impl IntoIterator<Item = &'a ParamMeta>,
	) {
		self.declarations.push(ParamsDeclaration {
			declarer: ParamsDeclarer::Route,
			params: metas.into_iter().cloned().collect(),
		});
	}

	/// This union with `partial` added.
	pub fn with(mut self, partial: &ParamsPartial) -> Self {
		self.extend(partial);
		self
	}

	/// Each declaration in the order added.
	pub fn declarations(&self) -> &[ParamsDeclaration] { &self.declarations }

	/// Every declared param, a key declared twice appearing twice.
	pub fn params(&self) -> impl Iterator<Item = &ParamMeta> {
		self.declarations
			.iter()
			.flat_map(|declaration| declaration.params.iter())
	}

	/// Whether a declaration forwards every param it does not name, so no
	/// param is unknown, see [`ParamsPartial::forwarding`].
	pub fn forwards(&self) -> bool { self.forwards }

	/// Whether a request param named `key` is declared: its flag spelling
	/// (`dry-run`), its field spelling (`dry_run`) or a declared short flag.
	pub fn contains(&self, key: &str) -> bool {
		let flag = Self::flag(key);
		self.params().any(|item| {
			item.name() == flag
				|| item.short().is_some_and(|short| {
					let mut chars = key.chars();
					chars.next() == Some(short) && chars.next().is_none()
				})
		})
	}

	/// Refuses, as a `400` [`HttpError`], a request carrying a param nothing
	/// declares, with the [`unknown`](Self::unknown) message.
	pub fn reject_unknown(
		&self,
		params: &MultiMap<SmolStr, SmolStr>,
	) -> Result {
		match self.unknown(params) {
			Some(message) => Err(HttpError::bad_request(message).into()),
			None => Ok(()),
		}
	}

	/// The message naming every param `params` carries that nothing declares,
	/// each with the declared one it most plausibly misspells, so `--elevate`
	/// reads `did you mean --elevated?`. `None` when every param is declared,
	/// as is any param to a union holding a forwarding declaration.
	pub fn unknown(
		&self,
		params: &MultiMap<SmolStr, SmolStr>,
	) -> Option<String> {
		let mut unknown = params
			.keys()
			.filter(|key| !self.forwards && !self.contains(key))
			.map(|key| self.describe_unknown(key))
			.collect::<Vec<_>>();
		unknown.sort();
		match unknown.as_slice() {
			[] => None,
			[one] => Some(format!("unknown param {one}")),
			many => Some(format!(
				"unknown params:\n{}",
				many.iter()
					.map(|line| format!("- {line}"))
					.collect::<Vec<_>>()
					.join("\n")
			)),
		}
	}

	/// The message for a param the request got wrong: the [`ParamError`], then
	/// the param as `--help` lists it, its type and doc line, where something
	/// here declares it.
	pub fn explain(&self, err: &ParamError) -> String {
		match self.params().find(|meta| meta.name() == err.key()) {
			Some(meta) => format!("{err}\n{}", meta.summary()),
			None => err.to_string(),
		}
	}

	/// `params` less every one nothing declares: what a command line rebuilt
	/// from a request may repeat, since a typo the request carried is no
	/// instruction to pass on. A forwarding union keeps them all.
	pub fn recognised(
		&self,
		params: &MultiMap<SmolStr, SmolStr>,
	) -> MultiMap<SmolStr, SmolStr> {
		let mut params = params.clone();
		params.retain(|key, _| self.forwards || self.contains(key));
		params
	}

	/// One unknown param as the error names it, spelled as typed (`-x` for a
	/// short flag), with the closest declared one.
	fn describe_unknown(&self, key: &str) -> String {
		let dashes = match key.chars().count() {
			1 => "-",
			_ => "--",
		};
		match str_ext::closest(
			&Self::flag(key),
			self.params().map(|item| item.name()),
		) {
			Some(near) => format!("`{dashes}{key}`, did you mean `--{near}`?"),
			None => format!("`{dashes}{key}`"),
		}
	}

	/// A request param key in its flag spelling, `dry_run` as `dry-run`.
	fn flag(key: &str) -> String { key.replace('_', "-") }
}

/// Resolves the [`DeclaredParams`] at an entity: every [`ParamsPartial`] and
/// every [`DeclaresParams`] component on it and on each of its ancestors,
/// the whole path a request reaching it was dispatched through.
#[derive(SystemParam)]
pub struct DeclaredParamsQuery<'w, 's> {
	ancestors: Query<'w, 's, &'static ChildOf>,
	partials: Query<'w, 's, &'static ParamsPartial>,
	entities: Query<'w, 's, EntityRef<'static>>,
	components: &'w Components,
	registry: Option<Res<'w, AppTypeRegistry>>,
}

impl DeclaredParamsQuery<'_, '_> {
	/// The params declared at `entity` and above it.
	pub fn of(&self, entity: Entity) -> DeclaredParams {
		let registry = self.registry.as_ref().map(|registry| registry.read());
		let mut declared = DeclaredParams::default();
		for entity in self.ancestors.iter_ancestors_inclusive(entity) {
			if let Ok(partial) = self.partials.get(entity) {
				declared.extend(partial);
			}
			let (Some(registry), Ok(entity_ref)) =
				(registry.as_ref(), self.entities.get(entity))
			else {
				continue;
			};
			for (name, data) in entity_ref
				.archetype()
				.iter_components()
				.filter_map(|id| self.components.get_info(id)?.type_id())
				.filter_map(|type_id| registry.get(type_id))
				.filter_map(|registration| {
					(
						registration.type_info().type_path_table().short_path(),
						registration.data::<ReflectDeclaresParams>()?,
					)
						.xsome()
				}) {
				declared.extend_from(
					ParamsDeclarer::Component { entity, name },
					&data.declared_params(),
				);
			}
		}
		declared
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[derive(Reflect)]
	#[allow(dead_code)]
	struct DeployParams {
		elevated: bool,
		dry_run: bool,
	}

	#[derive(Reflect)]
	#[allow(dead_code)]
	struct VerboseParams {
		verbose: bool,
	}

	/// Stands in for middleware declaring the params it reads.
	#[derive(Default, Component, Reflect)]
	#[reflect(Component, DeclaresParams)]
	struct VerboseLogger;

	impl DeclaresParams for VerboseLogger {
		fn declared_params() -> ParamsPartial {
			ParamsPartial::new::<VerboseParams>()
		}
	}

	fn deploy() -> DeclaredParams {
		DeclaredParams::default().with(&ParamsPartial::new::<DeployParams>())
	}

	fn params(args: &str) -> MultiMap<SmolStr, SmolStr> {
		CliArgs::parse(args).params
	}

	#[beet_core::test]
	fn accepts_declared_params_in_either_spelling() {
		deploy()
			.reject_unknown(&params("--elevated --dry-run --dry_run"))
			.unwrap();
	}

	#[beet_core::test]
	fn suggests_the_closest_declared_param() {
		deploy()
			.unknown(&params("--elevate"))
			.unwrap()
			.xpect_eq("unknown param `--elevate`, did you mean `--elevated`?");
		deploy()
			.unknown(&params("--elevate --colour"))
			.unwrap()
			.xpect_eq(
				"unknown params:\n- `--colour`\n- `--elevate`, did you mean `--elevated`?",
			);
	}

	/// The refusal is a `400`, so a CLI exits with its usage-error code and a
	/// response carries the message alone.
	#[beet_core::test]
	fn rejects_as_a_bad_request() {
		let response = Response::from(
			deploy().reject_unknown(&params("--elevate")).unwrap_err(),
		);
		response.status().xpect_eq(StatusCode::BAD_REQUEST);
	}

	/// A param the request got wrong is named with its type and doc line, as
	/// `--help` lists it.
	#[beet_core::test]
	fn explains_a_param_error_by_its_declaration() {
		#[derive(Reflect)]
		#[allow(dead_code)]
		struct ServeParams {
			/// The port to listen on.
			port: u16,
		}
		let declared = DeclaredParams::default()
			.with(&ParamsPartial::new::<ServeParams>());
		declared.explain(&ParamError::missing("port")).xpect_eq(
			"missing required param `--port`\n`--port` (u16): The port to listen on.",
		);
		// an undeclared key has no declaration to read
		declared
			.explain(&ParamError::missing("host"))
			.xpect_eq("missing required param `--host`");
	}

	#[beet_core::test]
	fn forwarding_knows_no_param_as_unknown() {
		let declared = DeclaredParams::default()
			.with(&ParamsPartial::new::<DeployParams>().forwarding());
		declared.reject_unknown(&params("--nocapture")).unwrap();
		declared
			.recognised(&params("--nocapture"))
			.contains_key("nocapture")
			.xpect_true();
	}

	#[beet_core::test]
	fn recognised_drops_the_unknown() {
		let recognised = deploy().recognised(&params("--elevate --dry-run"));
		recognised.contains_key("dry-run").xpect_true();
		recognised.contains_key("elevate").xpect_false();
	}

	/// The query unions a route's own partial with every partial and
	/// [`DeclaresParams`] component above it.
	#[beet_core::test]
	fn collects_partials_and_declaring_components_above() {
		let mut world = World::new();
		world.init_resource::<AppTypeRegistry>();
		world
			.resource::<AppTypeRegistry>()
			.write()
			.register::<VerboseLogger>();
		let root = world.spawn(VerboseLogger).id();
		let declared = world
			.spawn((ChildOf(root), ParamsPartial::new::<DeployParams>()))
			.with_state::<DeclaredParamsQuery, _>(|route, query| {
				query.of(route)
			});
		declared.contains("verbose").xpect_true();
		declared.contains("elevated").xpect_true();
		declared.contains("roll").xpect_false();
		// each kept beside its declarer, for `--help` to group by
		declared
			.declarations()
			.iter()
			.map(|declaration| declaration.declarer)
			.collect::<Vec<_>>()
			.xpect_eq(vec![ParamsDeclarer::Route, ParamsDeclarer::Component {
				entity: root,
				name: "VerboseLogger",
			}]);
	}
}
