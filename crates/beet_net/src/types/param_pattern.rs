use super::*;
use beet_core::prelude::*;
use bevy::reflect::TypeInfo;
use bevy::reflect::Typed;
use core::any::TypeId;
use heck::ToKebabCase;

/// The params one route takes: the union over every extractor that runs for
/// it, each param beside every source declaring it, so `--help`, the strict refusal
/// and the `400` explanation all read the one pattern.
///
/// Collected per route by the router from the [`RouteParams`] on the route
/// and its steps and the [`SubtreeParams`] on the route and every ancestor.
/// Two sources naming one key must agree on its type and arity, see
/// [`from_declarations`](Self::from_declarations).
#[derive(Debug, Default, Clone, PartialEq)]
pub struct ParamsPattern {
	/// Each param once, sorted by key.
	items: Vec<PatternParam>,
	/// Every source, in the order the collector supplied them.
	sources: Vec<ParamsSource>,
	/// Whether any source takes any param, see [`AnyParams`].
	takes_any: bool,
}

/// One param of a [`ParamsPattern`], beside every source declaring it.
#[derive(Debug, Clone, PartialEq)]
pub struct PatternParam {
	/// The param, as its first source declared it.
	pub meta: ParamMeta,
	/// Every source declaring it.
	pub sources: Vec<ParamsSource>,
}

/// Where a [`ParamsPattern`]'s params come from: one extractor's declaration
/// on one entity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParamsSource {
	/// The entity the extractor sits on.
	pub entity: Entity,
	/// The extractor's short type name, ie `CliServer`.
	pub name: SmolStr,
	/// Which list the declaration came from.
	pub scope: ParamsScope,
}

impl ParamsPattern {
	/// The pattern of `declarations`, each the entity it sits on, its scope
	/// and what it declares.
	///
	/// ## Errors
	///
	/// Errors when two sources name one key with a different type or arity.
	/// One key of one type declared twice is two extractors of the same param,
	/// kept once with both sources.
	pub fn from_declarations<'a>(
		declarations: impl IntoIterator<
			Item = (Entity, ParamsScope, &'a ParamsDeclaration),
		>,
	) -> Result<Self> {
		let mut pattern = Self::default();
		for (entity, scope, declaration) in declarations {
			let source = ParamsSource {
				entity,
				name: declaration.name().clone(),
				scope,
			};
			pattern.takes_any |= declaration.takes_any();
			for meta in declaration.metas() {
				pattern.add(&source, meta)?;
			}
			if !pattern.sources.contains(&source) {
				pattern.sources.push(source);
			}
		}
		pattern
			.items
			.sort_by(|left, right| left.meta.key.cmp(&right.meta.key));
		pattern.xok()
	}

	/// Add `meta` declared by `source`, applying the conflict rule.
	fn add(&mut self, source: &ParamsSource, meta: &ParamMeta) -> Result {
		let Some(item) =
			self.items.iter_mut().find(|item| item.meta.key == meta.key)
		else {
			self.items.push(PatternParam {
				meta: meta.clone(),
				sources: vec![source.clone()],
			});
			return Ok(());
		};
		if item.meta.type_path != meta.type_path
			|| item.meta.value != meta.value
		{
			let first = &item.sources[0].name;
			bevybail!(
				"conflicting declarations of `--{key}`: `{first}` takes {} ({}), `{}` takes {} ({})",
				item.meta.type_path,
				item.meta.value,
				source.name,
				meta.type_path,
				meta.value,
				key = meta.key,
			);
		}
		if !item.sources.contains(source) {
			item.sources.push(source.clone());
		}
		Ok(())
	}

	/// Each param once, sorted by key, beside its sources.
	pub fn items(&self) -> &[PatternParam] { &self.items }

	/// Each declared param once, sorted by key.
	pub fn params(&self) -> impl Iterator<Item = &ParamMeta> {
		self.items.iter().map(|item| &item.meta)
	}

	/// Whether any source takes any param, so no param is unknown, see
	/// [`AnyParams`].
	pub fn takes_any(&self) -> bool { self.takes_any }

	/// The params the route itself takes, its own extractors' and its steps'
	/// ([`ParamsScope::Route`]), sorted by key.
	pub fn own(&self) -> impl Iterator<Item = &ParamMeta> {
		self.items
			.iter()
			.filter(|item| {
				item.sources
					.iter()
					.any(|source| source.scope == ParamsScope::Route)
			})
			.map(|item| &item.meta)
	}

	/// Each subtree source ([`ParamsScope::Subtree`]) in collector order,
	/// beside the params it takes, sorted by key.
	pub fn inherited(
		&self,
	) -> impl Iterator<Item = (&ParamsSource, Vec<&ParamMeta>)> {
		self.sources
			.iter()
			.filter(|source| source.scope == ParamsScope::Subtree)
			.map(|source| {
				let params = self
					.items
					.iter()
					.filter(|item| item.sources.contains(source))
					.map(|item| &item.meta)
					.collect();
				(source, params)
			})
	}

	/// Whether a request param named `key` is declared: its flag spelling
	/// (`dry-run`), its field spelling (`dry_run`) or a declared short flag.
	pub fn contains(&self, key: &str) -> bool {
		let flag = Self::flag(key);
		self.params().any(|meta| {
			meta.name() == flag
				|| meta.short().is_some_and(|short| {
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
	/// as is any param to a pattern taking any.
	pub fn unknown(
		&self,
		params: &MultiMap<SmolStr, SmolStr>,
	) -> Option<String> {
		let mut unknown = params
			.keys()
			.filter(|key| !self.takes_any && !self.contains(key))
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
	/// instruction to pass on. A pattern taking any keeps them all.
	pub fn recognised(
		&self,
		params: &MultiMap<SmolStr, SmolStr>,
	) -> MultiMap<SmolStr, SmolStr> {
		let mut params = params.clone();
		params.retain(|key, _| self.takes_any || self.contains(key));
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
			self.params().map(|meta| meta.name()),
		) {
			Some(near) => format!("`{dashes}{key}`, did you mean `--{near}`?"),
			None => format!("`{dashes}{key}`"),
		}
	}

	/// A request param key in its flag spelling, `dry_run` as `dry-run`.
	fn flag(key: &str) -> String { key.replace('_', "-") }
}

/// Metadata for a specific param at a route
///
/// Field names are automatically converted from snake_case to kebab-case for
/// use in CLI arguments and query parameters. This means a struct field named
/// `help_format` will be accessible as `--help-format` in CLI and `help-format`
/// in query parameters.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
pub struct ParamMeta {
	/// The kebab-case key of the route param, ie the `foo-bar` in `--foo-bar=3`.
	/// Automatically converted from snake_case field names (e.g., `help_format` → `help-format`).
	key: String,
	/// The param value arity (flag / single / multiple).
	value: ParamValue,
	/// The concrete Rust type the param parses into, eg `alloc::string::String`,
	/// with any `Option<..>` wrapper stripped (its optionality is conveyed by
	/// [`required`](Self::required)). This is the param's displayed `kind`.
	type_path: String,
	/// Additional details for the param
	options: ParamOptions,
	/// Whether specifying the param is required, usually inferred
	/// by presence of `Option`
	required: bool,
}

impl core::fmt::Display for ParamMeta {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		write!(f, "Param - name: {}", self.key)?;
		if let Some(short) = self.short() {
			write!(f, ", short: -{}", short)?;
		}
		write!(
			f,
			", required: {}, kind: {}",
			self.is_required(),
			self.type_path
		)?;
		if let Some(desc) = self.description() {
			write!(f, ", description: {}", desc)?;
		}
		Ok(())
	}
}

impl ParamMeta {
	/// Creates a new `ParamMeta` with the given name and value arity, deriving a
	/// representative concrete type for the arity (no reflected field to read).
	pub fn new(name: impl Into<String>, value: ParamValue) -> Self {
		Self {
			type_path: value.placeholder_type_path().to_string(),
			value,
			key: name.into(),
			options: default(),
			required: false,
		}
	}

	/// Creates a `ParamMeta` from a reflected struct field, reading the field
	/// type exactly as [`RequestParts::parse_params`] does: an `Option<T>` is
	/// documented as its `T` and never required, a `bool` is a flag, a list
	/// repeats, and anything else must be supplied.
	pub fn from_field(field: &bevy::reflect::NamedField) -> Self {
		let info = field.type_info();
		let some_inner = info.and_then(reflect_ext::option_some_inner);
		let leaf_id = some_inner.map_or(field.type_id(), TypeInfo::type_id);
		let value = ParamValue::of_leaf(leaf_id, some_inner.or(info));

		Self {
			key: field.name().to_kebab_case(),
			required: value == ParamValue::Single && some_inner.is_none(),
			value,
			type_path: authored_kind(leaf_id)
				.or_else(|| variant_kind(some_inner.or(info)))
				.unwrap_or_else(|| kind_from_type_path(field.type_path())),
			options: ParamOptions::from_reflect(field),
		}
	}

	/// One meta per field of `T`: a struct, or a tuple of structs walked
	/// recursively, a nested struct with no authored form flattened into its
	/// parent exactly as [`RequestParts::parse_params`] reads it, and an
	/// [`AnyParams`] member contributing none.
	///
	/// ## Panics
	///
	/// Panics if `T` is not a struct or tuple of structs, or a field is missing
	/// its type info.
	pub fn of<T: FromReflect + Typed>() -> Vec<ParamMeta> {
		let mut metas = Vec::new();
		Self::walk(T::type_info(), &mut metas).unwrap();
		metas
	}

	/// Push one meta per field of the params type `type_info` describes,
	/// returning whether a member is [`AnyParams`].
	pub(crate) fn walk(
		type_info: &TypeInfo,
		metas: &mut Vec<ParamMeta>,
	) -> Result<bool> {
		match type_info {
			_ if AnyParams::is(type_info) => Ok(true),
			TypeInfo::Struct(info) => {
				for field in info.iter() {
					match field.type_info() {
						// a nested struct with no authored form is read
						// flattened into this one's namespace, so declared so
						Some(nested @ TypeInfo::Struct(_))
							if LiteralParser::get(field.type_id())
								.is_none() =>
						{
							Self::walk(nested, metas)?;
						}
						_ => metas.push(Self::from_field(field)),
					}
				}
				Ok(false)
			}
			TypeInfo::Tuple(info) => {
				let mut takes_any = false;
				for field in info.iter() {
					let field = field
						.type_info()
						.ok_or_else(|| bevyhow!("Field has no type info"))?;
					takes_any |= Self::walk(field, metas)?;
				}
				Ok(takes_any)
			}
			_ => bevybail!(
				"`{}` is no params type, only structs and tuples of structs are",
				type_info.type_path()
			),
		}
	}

	/// The name of the param
	pub fn name(&self) -> &str { &self.key }

	/// The concrete Rust type the param parses into, the displayed `kind`
	/// (an `Option<..>` wrapper is stripped, its optionality given by
	/// [`is_required`](Self::is_required)).
	pub fn type_path(&self) -> &str { &self.type_path }

	/// The description of the param
	pub fn description(&self) -> Option<&str> {
		self.options.description.as_deref()
	}

	/// Marks this param as required.
	pub fn required(mut self) -> Self {
		self.required = true;
		self
	}

	/// Sets the concrete type path shown as the param's `kind`.
	pub fn with_type_path(mut self, type_path: impl Into<String>) -> Self {
		self.type_path = type_path.into();
		self
	}

	/// Sets a single-character short flag for this param.
	pub fn with_short(mut self, short: char) -> Self {
		self.options.short = Some(short);
		self
	}
	/// Sets a description for this param.
	pub fn with_description(mut self, description: impl Into<String>) -> Self {
		self.options.description = Some(description.into());
		self
	}

	/// The param on one line as `--help` lists it: its flag, type and doc
	/// line, ie `--port` (u16): The port to listen on.
	pub fn summary(&self) -> String {
		let short = self
			.short()
			.map(|short| format!(", `-{short}`"))
			.unwrap_or_default();
		let head = format!("`--{}`{short} ({})", self.key, self.type_path);
		match self.description() {
			Some(description) => format!("{head}: {description}"),
			None => head,
		}
	}

	/// The short character representation
	pub fn short(&self) -> Option<char> { self.options.short }

	/// Whether the param is required
	pub fn is_required(&self) -> bool { self.required }

	/// The param value type
	pub fn value(&self) -> &ParamValue { &self.value }
}

/// The kind of value a param takes
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
pub enum ParamValue {
	/// A simple flag, ie `beet foo --bar`
	Flag,
	/// A single value, ie `beet foo --bar=1`
	Single,
	/// Multiple items allowed, ie `beet foo --bar=1 --bar=2`
	Multiple,
}

impl core::fmt::Display for ParamValue {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		match self {
			Self::Flag => write!(f, "flag"),
			Self::Single => write!(f, "single"),
			Self::Multiple => write!(f, "multiple"),
		}
	}
}

impl ParamValue {
	/// The arity of a leaf type (any `Option` wrapper already seen through):
	/// a `bool` is a flag, a list repeats, anything else takes one value.
	fn of_leaf(type_id: TypeId, info: Option<&TypeInfo>) -> Self {
		match info {
			_ if type_id == TypeId::of::<bool>() => Self::Flag,
			Some(TypeInfo::List(_)) => Self::Multiple,
			_ => Self::Single,
		}
	}

	/// A representative concrete type path for this arity, used when a
	/// [`ParamMeta`] is built without a reflected field (eg [`ParamMeta::new`]).
	fn placeholder_type_path(&self) -> &'static str {
		match self {
			Self::Flag => "bool",
			Self::Single => "alloc::string::String",
			Self::Multiple => "alloc::vec::Vec<alloc::string::String>",
		}
	}
}

/// How a param's leaf type (any `Option` wrapper seen through) is WRITTEN,
/// when its [`LiteralParser`] says, so a `--created` flag documents itself as
/// `a YYYY-MM-DD date` rather than as `beet_core::utils::timestamp::Timestamp`.
///
/// `None` for a type with no parser or no hint, which falls back to the type
/// path: for a structural type that path is the only clue to what to write.
fn authored_kind(type_id: TypeId) -> Option<String> {
	LiteralParser::get(type_id)?.hint().map(str::to_string)
}

/// How a unit-only enum is written: its variants as the kebab-case words a
/// flag takes, ie `one of: main, content` for a `--root`, since the
/// type path names none of them.
fn variant_kind(info: Option<&TypeInfo>) -> Option<String> {
	let Some(TypeInfo::Enum(info)) = info else {
		return None;
	};
	info.iter()
		.map(|variant| match variant {
			bevy::reflect::enums::VariantInfo::Unit(unit) => {
				Some(unit.name().to_kebab_case())
			}
			_ => None,
		})
		.collect::<Option<Vec<_>>>()
		.filter(|variants| !variants.is_empty())
		.map(|variants| format!("one of: {}", variants.join(", ")))
}

/// The concrete type path shown as a param's `kind`, with a `core::option::Option<..>`
/// wrapper stripped (its optionality is conveyed by the param's `required` flag).
fn kind_from_type_path(type_path: &str) -> String {
	type_path
		.strip_prefix("core::option::Option<")
		.and_then(|inner| inner.strip_suffix('>'))
		.unwrap_or(type_path)
		.to_string()
}

/// Additional options for parameter metadata.
#[derive(
	Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect,
)]
pub struct ParamOptions {
	/// A description of the route param, usually the doc comment of its
	/// field on the params type.
	description: Option<String>,
	/// A single character short flag, ie the `r` in `-r`.
	short: Option<char>,
}

impl ParamOptions {
	/// Creates options with only a description.
	pub fn desc(description: impl Into<String>) -> Self {
		Self {
			description: Some(description.into()),
			short: None,
		}
	}

	/// Creates options with both a description and short flag.
	pub fn desc_and_short(description: impl Into<String>, short: char) -> Self {
		Self {
			description: Some(description.into()),
			short: Some(short),
		}
	}

	/// Creates options with only a short flag.
	pub fn short(short: char) -> Self {
		Self {
			description: None,
			short: Some(short),
		}
	}

	fn from_reflect(field: &bevy::reflect::NamedField) -> Self {
		#[allow(unused_mut)]
		let mut opts = field.get_attribute::<Self>().cloned().unwrap_or_default();
		// the doc comment describes a param the attribute leaves undescribed,
		// its wrapped lines joined into the one line a help row holds
		#[cfg(feature = "bevy_reflect_documentation")]
		if opts.description.is_none() {
			opts.description = field.docs().map(|docs| {
				docs.split_whitespace().collect::<Vec<_>>().join(" ")
			});
		}
		opts
	}
}

#[cfg(test)]
mod test {
	use super::*;

	/// A unit-only enum documents the words it takes.
	#[beet_core::test]
	fn unit_enum_lists_its_variants() {
		#[derive(Reflect)]
		#[allow(dead_code)]
		enum Shape {
			Round,
			SharpEdged,
		}
		#[derive(Reflect)]
		#[allow(dead_code)]
		struct Params {
			shape: Option<Shape>,
		}
		let TypeInfo::Struct(info) = Params::type_info() else {
			panic!("expected struct");
		};
		ParamMeta::from_field(info.field("shape").unwrap())
			.type_path()
			.xpect_eq("one of: round, sharp-edged");
	}

	/// A reflected field's concrete type is captured as the param `kind`, with an
	/// `Option<..>` wrapper stripped (the option drives `required`, not `kind`).
	#[beet_core::test]
	fn from_field_captures_concrete_type() {
		#[derive(Reflect)]
		#[allow(dead_code)]
		struct Params {
			package: String,
			out_dir: Option<String>,
			release: bool,
		}
		let TypeInfo::Struct(info) = Params::type_info() else {
			panic!("expected struct");
		};
		let by_name =
			|name: &str| ParamMeta::from_field(info.field(name).unwrap());
		// a required `String` shows its concrete type, not the `single` arity
		let package = by_name("package");
		package.type_path().xpect_eq("alloc::string::String");
		package.is_required().xpect_true();
		// an `Option<String>` is not required and its `kind` strips the wrapper
		let out_dir = by_name("out_dir");
		out_dir.type_path().xpect_eq("alloc::string::String");
		out_dir.is_required().xpect_false();
		// a `bool` is a flag whose kind is `bool`
		by_name("release").type_path().xpect_eq("bool");
		// the Display form now carries the concrete type as `kind`
		package
			.to_string()
			.xpect_contains("kind: alloc::string::String");
	}

	/// A field whose type is AUTHORED as text documents itself that way, since
	/// its Rust path says nothing about what to type. Through an `Option`
	/// wrapper too, which changes only whether the flag is required.
	#[beet_core::test]
	fn from_field_prefers_the_authored_spelling() {
		#[derive(Reflect)]
		#[allow(dead_code)]
		struct Params {
			timeout: core::time::Duration,
			created: Option<Date>,
			label: String,
		}
		let TypeInfo::Struct(info) = Params::type_info() else {
			panic!("expected struct");
		};
		let by_name =
			|name: &str| ParamMeta::from_field(info.field(name).unwrap());
		by_name("timeout")
			.type_path()
			.xpect_eq("a unit-suffixed duration, eg \"30s\"");
		by_name("created")
			.type_path()
			.xpect_eq("a `YYYY-MM-DD` date");
		// a type whose own name says how it is written keeps its path
		by_name("label")
			.type_path()
			.xpect_eq("alloc::string::String");
	}

	#[derive(Reflect)]
	#[allow(dead_code)]
	struct DeployParams {
		elevated: bool,
		dry_run: bool,
	}

	/// The pattern of `declarations`, each read by a route on a placeholder.
	fn pattern(declarations: &[ParamsDeclaration]) -> Result<ParamsPattern> {
		ParamsPattern::from_declarations(declarations.iter().map(
			|declaration| {
				(Entity::PLACEHOLDER, ParamsScope::Route, declaration)
			},
		))
	}

	fn deploy() -> ParamsPattern {
		pattern(&[ParamsDeclaration::new::<DeployParams, DeployParams>()])
			.unwrap()
	}

	fn params(args: &str) -> MultiMap<SmolStr, SmolStr> {
		CliArgs::parse(args).params
	}

	#[beet_core::test]
	fn accepts_declared_keys_in_either_spelling() {
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
		let pattern =
			pattern(&[ParamsDeclaration::new::<ServeParams, ServeParams>()])
				.unwrap();
		pattern.explain(&ParamError::missing("port")).xpect_eq(
			"missing required param `--port`\n`--port` (u16): The port to listen on.",
		);
		// an undeclared key has no declaration to read
		pattern
			.explain(&ParamError::missing("host"))
			.xpect_eq("missing required param `--host`");
	}

	/// A pattern taking any param knows none as unknown, and keeps every one
	/// it recognises.
	#[beet_core::test]
	fn any_params_refuses_nothing() {
		let pattern = pattern(&[ParamsDeclaration::new::<
			DeployParams,
			(DeployParams, AnyParams),
		>()])
		.unwrap();
		pattern.takes_any().xpect_true();
		pattern.reject_unknown(&params("--nocapture")).unwrap();
		pattern
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

	/// One key of two types is a contradiction; one key of one type is two
	/// extractors of the same param, kept once beside both.
	#[beet_core::test]
	fn conflicts_on_type_not_on_sources() {
		let port = |name: &str, type_path: &str| {
			ParamsDeclaration::named(
				name,
				[ParamMeta::new("port", ParamValue::Single)
					.with_type_path(type_path)],
				false,
			)
		};
		pattern(&[
			port("HttpServer", "u16"),
			port("Serve", "alloc::string::String"),
		])
		.unwrap_err()
		.to_string()
		.xpect_contains("conflicting declarations of `--port`");
		let pattern =
			pattern(&[port("HttpServer", "u16"), port("TuiServer", "u16")])
				.unwrap();
		pattern.items().len().xpect_eq(1);
		pattern.items()[0]
			.sources
			.iter()
			.map(|source| source.name.as_str())
			.collect::<Vec<_>>()
			.xpect_eq(vec!["HttpServer", "TuiServer"]);
	}

	#[beet_core::test]
	fn from_reflect() {
		#[derive(Reflect)]
		struct MyParams {
			foo: u32,
			#[reflect(@ParamOptions::desc("all about 'bar'"))]
			bar: Option<String>,
			#[reflect(@ParamOptions::desc_and_short("all about 'bazz'",'b'))]
			bazz: Vec<f64>,
			boo: bool,
		}

		ParamMeta::of::<MyParams>().xpect_eq(vec![
			ParamMeta::new("foo", ParamValue::Single)
				.with_type_path("u32")
				.required(),
			ParamMeta::new("bar", ParamValue::Single)
				.with_type_path("alloc::string::String")
				.with_description("all about 'bar'"),
			ParamMeta::new("bazz", ParamValue::Multiple)
				.with_type_path("alloc::vec::Vec<f64>")
				.with_description("all about 'bazz'")
				.with_short('b'),
			// .required(),
			ParamMeta::new("boo", ParamValue::Flag).with_type_path("bool"),
		]);
	}

	/// What `--help` says a flag is, the typed read enforces: over a params
	/// type with every kind, a param is required exactly when leaving it out
	/// fails [`RequestParts::parse_params`], and the failure names it.
	#[beet_core::test]
	fn help_and_parse_agree_on_required() {
		#[derive(Reflect)]
		#[allow(dead_code)]
		struct Params {
			name: String,
			store: StoreUri,
			out: Option<String>,
			release: bool,
			verbose: Option<bool>,
			tags: Vec<String>,
		}
		let metas = ParamMeta::of::<Params>();
		metas
			.iter()
			.filter(|meta| meta.is_required())
			.map(ParamMeta::name)
			.collect::<Vec<_>>()
			.xpect_eq(vec!["name", "store"]);
		for meta in metas.iter() {
			// every required param but `meta` supplied
			let args = metas
				.iter()
				.filter(|other| {
					other.is_required() && other.name() != meta.name()
				})
				.map(|other| format!("--{}=memory://m", other.name()))
				.collect::<Vec<_>>()
				.join(" ");
			let parsed = Request::from_cli_str(&args).parse_params::<Params>();
			match meta.is_required() {
				true => {
					parsed
						.err()
						.unwrap()
						.to_string()
						.xpect_contains(format!("`--{}`", meta.name()));
				}
				false => {
					parsed.ok().unwrap();
				}
			}
		}
	}

	/// A nested struct is declared flattened, as the parse reads it, so a
	/// strict server accepts the flags the route reads.
	#[beet_core::test]
	fn nested_structs_declare_flattened() {
		#[derive(Debug, PartialEq, Reflect)]
		struct Inner {
			depth: u32,
		}
		#[derive(Debug, PartialEq, Reflect)]
		struct Outer {
			name: String,
			inner: Inner,
		}
		ParamMeta::of::<Outer>()
			.iter()
			.map(ParamMeta::name)
			.collect::<Vec<_>>()
			.xpect_eq(vec!["name", "depth"]);
		Request::from_cli_str("--name=x --depth=3")
			.parse_params::<Outer>()
			.unwrap()
			.xpect_eq(Outer {
				name: "x".into(),
				inner: Inner { depth: 3 },
			});
	}

	#[beet_core::test]
	fn snake_case_converts_to_kebab_case() {
		#[derive(Reflect)]
		struct SnakeCaseParams {
			help_format: Option<String>,
			enable_verbose_mode: bool,
			max_retry_count: u32,
		}

		let metas = ParamMeta::of::<SnakeCaseParams>();

		// verify field names are converted to kebab-case
		metas[0].name().xpect_eq("help-format");
		metas[1].name().xpect_eq("enable-verbose-mode");
		metas[2].name().xpect_eq("max-retry-count");
	}
}
