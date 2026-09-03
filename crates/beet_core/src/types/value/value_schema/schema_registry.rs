//! The by-name schema registry backing authored, composable and remote schemas.
//!
//! A [`ValueSchema::Reference`] names another schema rather than inlining it, so
//! schemas form a graph mirroring the template graph. [`SchemaRegistry`] is the
//! index that resolves those names, and it holds authored schemas (a `bx:schema`
//! block, a schema document) beside reflect-derived ones: one namespace, because
//! an authored schema and a reflect-derived one are the same kind of thing.
//!
//! Identity is the **full** key: a Rust type path, a template module path, an
//! authored name, or the store path of a schema document that declares no name. A short name is display and authoring sugar, indexed as an
//! alias; a short name several full keys share is ambiguous, diagnosed where the
//! collision is created and resolving to nothing rather than to whichever
//! registered first.
//!
//! References may resolve asynchronously (a remote `bx:schema`, a schema
//! document still being read), so a name that is not yet registered defers to
//! [`ValueSchema::Any`] rather than erroring.

use crate::prelude::*;
use bevy::reflect::Typed;

/// A by-name index of authored and type schemas, the manifest a reactive client
/// layer reads and the resolver for [`ValueSchema::Reference`].
///
/// Registered before any template is loaded (so a tag resolves to a known
/// schema), it is the schema-side companion of the template-by-name registry.
/// [`SchemaRegistry::located`] is the by-*location* index beside it, which a
/// schema document read out of a store registers into.
#[derive(Debug, Clone, Resource)]
pub struct SchemaRegistry {
	/// Keyed by full identity: a Rust type path, a template module path, or an
	/// authored name.
	schemas: HashMap<SmolStr, ValueSchema>,
	/// Short name to every full key ending in it. One key resolves, several are
	/// a collision.
	aliases: HashMap<SmolStr, Vec<SmolStr>>,
	/// A schema document's store path, to the key its schema is stored under.
	/// A pointer into `schemas` rather than a second copy, so the by-name and
	/// by-location views of one schema can never disagree.
	located: HashMap<SmolPath, SmolStr>,
}

impl Default for SchemaRegistry {
	fn default() -> Self { Self::new() }
}

impl SchemaRegistry {
	/// A registry holding only the meta-schema.
	///
	/// The meta-schema is intrinsic rather than opt-in: a schema is itself a
	/// value with a schema, so every registry can describe its own contents and
	/// a schema document validates wherever it is read.
	pub fn new() -> Self {
		let mut registry = Self {
			schemas: HashMap::default(),
			aliases: HashMap::default(),
			located: HashMap::default(),
		};
		registry.insert(ValueSchema::type_path(), ValueSchema::meta());
		registry
	}

	/// Register `schema` under `name`, its full identity key.
	///
	/// Re-registering a key replaces it, which is how a reloaded template dir or
	/// an edited schema document updates in place.
	pub fn insert(&mut self, name: impl Into<SmolStr>, schema: ValueSchema) {
		let name = name.into();
		let short = Self::short_name(&name);
		if short != name {
			let keys = self.aliases.entry(short.clone()).or_default();
			if !keys.contains(&name) {
				keys.push(name.clone());
				if keys.len() > 1 {
					warn!(
						"schema short name `{short}` is ambiguous between {}; \
						reference one of them by its full path",
						keys.join(", ")
					);
				}
			}
		}
		self.schemas.insert(name, schema);
	}

	/// Register the reflect-derived schema of `T` under its full type path.
	pub fn register_type<T: TypePath + Typed>(&mut self) {
		self.insert(T::type_path(), ValueSchema::of::<T>());
	}

	/// Register the schema of the schema document at `path`, the by-location
	/// index a [`FieldSchema::Document`] resolves through.
	///
	/// The schema is stored **once**, in the one by-name namespace, under the
	/// name it declares for itself or, declaring none, under `path`; the
	/// by-location index holds that key alone. So an authored schema joins the
	/// namespace beside the reflect-derived ones (a `Reference("TodoItem")` in a
	/// document that only composes the row schema resolves to it), and editing
	/// it by either route is what the other route then reads.
	pub fn insert_located(
		&mut self,
		path: impl Into<SmolPath>,
		schema: ValueSchema,
	) {
		let path = path.into();
		let key = match schema.name() {
			Some(name) => name.clone(),
			None => SmolStr::from(path.as_str()),
		};
		self.insert(key.clone(), schema);
		self.located.insert(path, key);
	}

	/// The schema of the schema document at `path`, if it has arrived.
	pub fn located(&self, path: &SmolPath) -> Option<&ValueSchema> {
		self.located.get(path).and_then(|key| self.schemas.get(key))
	}

	/// The raw (still-referencing) schema registered under `name`, matching the
	/// full key first and a short-name alias second.
	///
	/// An ambiguous short name resolves to `None`: a collision is never resolved
	/// first-wins. Use [`try_get`](Self::try_get) to fail loudly instead.
	pub fn get(&self, name: &str) -> Option<&ValueSchema> {
		self.schemas.get(name).or_else(|| {
			self.unique_alias(name)
				.and_then(|key| self.schemas.get(key))
		})
	}

	/// [`get`](Self::get), naming both candidates when `name` is an ambiguous
	/// short name.
	pub fn try_get(&self, name: &str) -> Result<Option<&ValueSchema>> {
		if self.schemas.contains_key(name) {
			return Ok(self.schemas.get(name));
		}
		match self.aliases.get(name).map(Vec::as_slice) {
			Some([key]) => Ok(self.schemas.get(key)),
			Some(keys) => bevybail!(
				"schema short name `{name}` is ambiguous between {}",
				keys.join(", ")
			),
			None => Ok(None),
		}
	}

	/// Whether a schema is registered under `name`.
	pub fn contains(&self, name: &str) -> bool { self.get(name).is_some() }

	/// The number of registered schemas, the meta-schema included.
	pub fn len(&self) -> usize { self.schemas.len() }

	/// Whether the registry is empty, which a registry built by
	/// [`new`](Self::new) never is.
	pub fn is_empty(&self) -> bool { self.schemas.is_empty() }

	/// Every registered name paired with its schema.
	pub fn iter(&self) -> impl Iterator<Item = (&SmolStr, &ValueSchema)> {
		self.schemas.iter()
	}

	/// Resolve `schema` against this registry, replacing every
	/// [`ValueSchema::Reference`] with the named schema recursively.
	///
	/// An unregistered reference stays deferred as [`ValueSchema::Any`], so a
	/// schema still arriving validates as a wildcard until it does. A reference
	/// back to a name already being expanded is left as a reference, so a
	/// recursive schema (the meta-schema is one) resolves to a finite tree
	/// rather than expanding forever; validation follows it lazily instead.
	pub fn resolve(&self, schema: &ValueSchema) -> ValueSchema {
		self.resolve_inner(schema, &mut Vec::new())
	}

	/// Resolve a named reference directly, the entrypoint a tag resolution uses.
	///
	/// Returns [`ValueSchema::Any`] when the name is not (yet) registered.
	pub fn resolve_name(&self, name: &str) -> ValueSchema {
		match self.get(name) {
			Some(schema) => {
				self.resolve_inner(schema, &mut vec![SmolStr::from(name)])
			}
			None => ValueSchema::Any,
		}
	}

	/// The trailing `::` segment of `name`, its display and authoring sugar.
	fn short_name(name: &str) -> SmolStr {
		name.rsplit("::").next().unwrap_or(name).into()
	}

	/// The sole full key `short` aliases, or `None` when it aliases several.
	fn unique_alias(&self, short: &str) -> Option<&SmolStr> {
		match self.aliases.get(short)?.as_slice() {
			[key] => Some(key),
			_ => None,
		}
	}

	fn resolve_inner(
		&self,
		schema: &ValueSchema,
		visiting: &mut Vec<SmolStr>,
	) -> ValueSchema {
		match schema {
			ValueSchema::Reference(name) => {
				if visiting.iter().any(|visited| visited == name) {
					return schema.clone();
				}
				let Some(target) = self.get(name) else {
					return ValueSchema::Any;
				};
				visiting.push(name.clone());
				let resolved = self.resolve_inner(target, visiting);
				visiting.pop();
				resolved
			}
			ValueSchema::Optional(inner) => ValueSchema::Optional(Box::new(
				self.resolve_inner(inner, visiting),
			)),
			ValueSchema::List(list) => ValueSchema::List(ListSchema {
				item: Box::new(self.resolve_inner(&list.item, visiting)),
				min_items: list.min_items,
				max_items: list.max_items,
				unique: list.unique,
			}),
			ValueSchema::Map(map) => ValueSchema::Map(MapSchema {
				value: Box::new(self.resolve_inner(&map.value, visiting)),
			}),
			ValueSchema::Struct(struct_schema) => {
				ValueSchema::Struct(StructSchema {
					name: struct_schema.name.clone(),
					allow_additional: struct_schema.allow_additional,
					fields: struct_schema
						.fields
						.iter()
						.map(|field| NamedFieldSchema {
							key: field.key.clone(),
							required: field.required,
							label: field.label.clone(),
							description: field.description.clone(),
							on_missing: field.on_missing.clone(),
							schema: self.resolve_inner(&field.schema, visiting),
						})
						.collect(),
				})
			}
			ValueSchema::Tuple(tuple) => ValueSchema::Tuple(TupleSchema {
				name: tuple.name.clone(),
				fields: tuple
					.fields
					.iter()
					.map(|field| UnnamedFieldSchema {
						required: field.required,
						description: field.description.clone(),
						schema: self.resolve_inner(&field.schema, visiting),
					})
					.collect(),
			}),
			ValueSchema::Enum(enum_schema) => ValueSchema::Enum(EnumSchema {
				name: enum_schema.name.clone(),
				variants: enum_schema
					.variants
					.iter()
					.map(|variant| VariantSchema {
						name: variant.name.clone(),
						payload: variant.payload.as_ref().map(|payload| {
							self.resolve_inner(payload, visiting)
						}),
					})
					.collect(),
			}),
			// scalars and wildcards resolve to themselves
			other => other.clone(),
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[derive(Reflect)]
	struct Count(#[allow(dead_code)] u32);

	/// `{ label: String }`, an authored schema that names itself.
	fn todo_item() -> ValueSchema {
		ValueSchema::Struct(StructSchema {
			name: Some("TodoItem".into()),
			allow_additional: false,
			fields: vec![NamedFieldSchema::new(
				"label",
				ValueSchema::String(default()),
			)],
		})
	}

	#[crate::test]
	fn resolves_a_reference() {
		let mut registry = SchemaRegistry::default();
		registry.insert("Count", ValueSchema::of::<i64>());
		let schema = ValueSchema::Reference("Count".into());
		registry.resolve(&schema).xpect_eq(ValueSchema::of::<i64>());
	}

	#[crate::test]
	fn unresolved_reference_is_wildcard() {
		let registry = SchemaRegistry::default();
		registry
			.resolve(&ValueSchema::Reference("Missing".into()))
			.xpect_eq(ValueSchema::Any);
	}

	#[crate::test]
	fn resolves_nested_list_reference() {
		let mut registry = SchemaRegistry::default();
		registry.insert("Item", ValueSchema::of::<i64>());
		let schema = ValueSchema::List(ListSchema {
			item: Box::new(ValueSchema::Reference("Item".into())),
			min_items: None,
			max_items: None,
			unique: false,
		});
		let ValueSchema::List(list) = registry.resolve(&schema) else {
			panic!("expected list");
		};
		(*list.item).xpect_eq(ValueSchema::of::<i64>());
	}

	/// A reference back into a name already being expanded is left in place, so
	/// a recursive schema resolves to a finite tree.
	#[crate::test]
	fn cyclic_reference_terminates() {
		let mut registry = SchemaRegistry::default();
		registry.insert("A", ValueSchema::Reference("B".into()));
		registry.insert("B", ValueSchema::Reference("A".into()));
		registry
			.resolve_name("A")
			.xpect_eq(ValueSchema::Reference("A".into()));
	}

	/// A short name is sugar over the full key, which is the only identity.
	#[crate::test]
	fn a_short_name_aliases_its_full_key() {
		let mut registry = SchemaRegistry::default();
		registry.register_type::<Count>();
		registry.get(Count::type_path()).is_some().xpect_true();
		registry.get("Count").is_some().xpect_true();
	}

	/// Two full keys sharing a short name are a collision: it resolves to
	/// nothing rather than to whichever registered first, and asking loudly
	/// names both candidates.
	#[crate::test]
	fn an_ambiguous_short_name_never_first_wins() {
		let mut registry = SchemaRegistry::default();
		registry.insert("a::Item", ValueSchema::of::<i64>());
		registry.insert("b::Item", ValueSchema::of::<String>());
		registry.get("Item").is_none().xpect_true();
		registry
			.try_get("Item")
			.unwrap_err()
			.to_string()
			.xpect_contains("a::Item")
			.xpect_contains("b::Item");
		// each is still reachable by its full key
		registry
			.get("a::Item")
			.unwrap()
			.xpect_eq(ValueSchema::of::<i64>());
	}

	/// An authored schema and a reflect-derived one share the one namespace.
	#[crate::test]
	async fn an_authored_schema_sits_beside_a_derived_one() {
		let mut registry = SchemaRegistry::default();
		registry.register_type::<Count>();
		registry.insert(
			"TodoItem",
			ValueSchema::Struct(StructSchema {
				name: Some("TodoItem".into()),
				allow_additional: false,
				fields: vec![NamedFieldSchema::new(
					"label",
					ValueSchema::String(default()),
				)],
			}),
		);
		let resolver = SchemaResolver::default().with_schemas(&registry);
		ValueSchema::Reference("TodoItem".into())
			.validate_in(resolver, &mut value!({ "label": "buy milk" }))
			.await
			.is_empty()
			.xpect_true();
		ValueSchema::Reference("Count".into())
			.validate_in(resolver, &mut value!(7u64))
			.await
			.is_empty()
			.xpect_true();
	}

	/// A schema document that names itself joins the by-name namespace, so a
	/// document composing it by reference resolves without knowing where it is
	/// stored. One declaring no name is keyed by its path instead.
	#[crate::test]
	fn a_schema_document_is_keyed_by_its_name_or_its_path() {
		let mut registry = SchemaRegistry::default();
		registry.insert_located("schema/todo.json", todo_item());
		registry.get("TodoItem").unwrap().xpect_eq(todo_item());

		registry.insert_located("schema/count.json", ValueSchema::of::<i64>());
		registry
			.get("schema/count.json")
			.unwrap()
			.xpect_eq(ValueSchema::of::<i64>());
	}

	/// The by-location index points at the one stored schema rather than
	/// holding a copy, so editing it by name is what a read by location sees.
	#[crate::test]
	fn a_located_schema_is_stored_once() {
		let mut registry = SchemaRegistry::default();
		let path = SmolPath::from("schema/todo.json");
		registry.insert_located(path.clone(), todo_item());
		registry.insert("TodoItem", ValueSchema::of::<i64>());
		registry
			.located(&path)
			.unwrap()
			.xpect_eq(ValueSchema::of::<i64>());
	}

	/// A self-referential Rust type lowers to a reference by short type path,
	/// so registering it is what lets validation follow the recursion instead of
	/// deferring one level in.
	#[crate::test]
	async fn a_recursive_type_validates_to_the_depth_of_its_data() {
		#[derive(Reflect)]
		#[allow(dead_code)]
		struct SidebarNode {
			label: String,
			children: Vec<SidebarNode>,
		}
		let mut registry = SchemaRegistry::default();
		registry.register_type::<SidebarNode>();
		let resolver = SchemaResolver::default().with_schemas(&registry);
		let schema = ValueSchema::of::<SidebarNode>();
		schema
			.validate_in(
				resolver,
				&mut value!({
					"label": "root",
					"children": [{ "label": "child", "children": [] }],
				}),
			)
			.await
			.is_empty()
			.xpect_true();
		// the nested level is really validated, not swallowed as a wildcard
		schema
			.validate_in(
				resolver,
				&mut value!({
					"label": "root",
					"children": [{ "children": [] }],
				}),
			)
			.await
			.is_empty()
			.xpect_false();
	}

	#[crate::test]
	async fn resolved_composable_schema_validates() {
		// a `todos` list of `TodoItem`, the composable case spanning two schemas
		let mut registry = SchemaRegistry::default();
		registry.insert(
			"TodoItem",
			ValueSchema::Struct(StructSchema {
				name: Some("TodoItem".into()),
				allow_additional: false,
				fields: vec![NamedFieldSchema::new(
					"label",
					ValueSchema::String(StringSchema::default()),
				)],
			}),
		);
		let list_schema = ValueSchema::List(ListSchema {
			item: Box::new(ValueSchema::Reference("TodoItem".into())),
			min_items: None,
			max_items: None,
			unique: false,
		});
		let resolved = registry.resolve(&list_schema);
		// a valid list of todo items passes
		resolved
			.validate(&mut value!([{ "label": "buy milk" }]))
			.await
			.is_empty()
			.xpect_true();
		// a todo item missing its required `label` fails recursively
		resolved
			.validate(&mut value!([{}]))
			.await
			.is_empty()
			.xpect_false();
	}
}
