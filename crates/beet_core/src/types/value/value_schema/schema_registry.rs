//! The by-name schema registry backing composable and remote schemas.
//!
//! A [`ValueSchema::Reference`] names another template's (or registered type's)
//! schema rather than inlining it, so schemas form a graph mirroring the
//! template graph. [`SchemaRegistry`] is the index that resolves those names,
//! and [`SchemaRegistry::resolve`] walks a schema, replacing every reference with
//! the registered schema recursively, so the result validates without further
//! lookups.
//!
//! References may resolve asynchronously (a remote `bx:schema`), so a name that
//! is not yet registered resolves to [`ValueSchema::Any`] (a wildcard that defers
//! validation) rather than erroring. The async layer registers the real schema
//! when it arrives and re-resolves.

use crate::prelude::*;

/// A by-name index of template/type schemas, the manifest a reactive client
/// layer reads and the resolver for [`ValueSchema::Reference`].
///
/// Registered before any template is loaded (so a tag resolves to a known
/// schema), it is the schema-side companion of the template-by-name registry.
#[derive(Debug, Default, Clone, Resource)]
pub struct SchemaRegistry {
	schemas: HashMap<SmolStr, ValueSchema>,
}

impl SchemaRegistry {
	/// Register `schema` under `name` (a template module path or short type path).
	pub fn insert(&mut self, name: impl Into<SmolStr>, schema: ValueSchema) {
		self.schemas.insert(name.into(), schema);
	}

	/// The raw (still-referencing) schema registered under `name`, if any.
	pub fn get(&self, name: &str) -> Option<&ValueSchema> {
		self.schemas.get(name)
	}

	/// Whether a schema is registered under `name`.
	pub fn contains(&self, name: &str) -> bool {
		self.schemas.contains_key(name)
	}

	/// The number of registered schemas.
	pub fn len(&self) -> usize { self.schemas.len() }

	/// Whether the registry is empty.
	pub fn is_empty(&self) -> bool { self.schemas.is_empty() }

	/// Resolve `schema` against this registry, replacing every
	/// [`ValueSchema::Reference`] with the named schema recursively.
	///
	/// An unregistered reference resolves to [`ValueSchema::Any`] (deferred), so a
	/// schema still resolving remotely validates as a wildcard until it arrives.
	/// Recursion is bounded by `depth` so a cyclic reference graph (`A -> B -> A`)
	/// terminates at a wildcard rather than looping.
	pub fn resolve(&self, schema: &ValueSchema) -> ValueSchema {
		self.resolve_inner(schema, MAX_RESOLVE_DEPTH)
	}

	/// Resolve a named reference directly, the entrypoint a tag resolution uses.
	///
	/// Returns [`ValueSchema::Any`] when the name is not (yet) registered.
	pub fn resolve_name(&self, name: &str) -> ValueSchema {
		match self.schemas.get(name) {
			Some(schema) => self.resolve_inner(schema, MAX_RESOLVE_DEPTH),
			None => ValueSchema::Any,
		}
	}

	fn resolve_inner(&self, schema: &ValueSchema, depth: usize) -> ValueSchema {
		if depth == 0 {
			return ValueSchema::Any;
		}
		match schema {
			ValueSchema::Reference(name) => {
				match self.schemas.get(name.as_str()) {
					Some(target) => self.resolve_inner(target, depth - 1),
					None => ValueSchema::Any,
				}
			}
			ValueSchema::Optional(inner) => ValueSchema::Optional(Box::new(
				self.resolve_inner(inner, depth - 1),
			)),
			ValueSchema::List(list) => ValueSchema::List(ListSchema {
				item: Box::new(self.resolve_inner(&list.item, depth - 1)),
				min_items: list.min_items,
				max_items: list.max_items,
				unique: list.unique,
			}),
			ValueSchema::Map(map) => ValueSchema::Map(MapSchema {
				value: Box::new(self.resolve_inner(&map.value, depth - 1)),
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
							schema: self
								.resolve_inner(&field.schema, depth - 1),
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
						schema: self.resolve_inner(&field.schema, depth - 1),
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
							self.resolve_inner(payload, depth - 1)
						}),
					})
					.collect(),
			}),
			// scalars and wildcards resolve to themselves
			other => other.clone(),
		}
	}
}

/// Bound on [`SchemaRegistry::resolve`] recursion, so a cyclic schema graph
/// terminates at a wildcard rather than overflowing the stack.
const MAX_RESOLVE_DEPTH: usize = 64;

#[cfg(test)]
mod test {
	use super::*;

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

	#[crate::test]
	fn cyclic_reference_terminates() {
		let mut registry = SchemaRegistry::default();
		// A references B, B references A
		registry.insert("A", ValueSchema::Reference("B".into()));
		registry.insert("B", ValueSchema::Reference("A".into()));
		// terminates at a wildcard rather than looping
		matches!(registry.resolve_name("A"), ValueSchema::Any).xpect_true();
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
				fields: vec![NamedFieldSchema {
					key: "label".into(),
					required: true,
					label: None,
					description: None,
					schema: ValueSchema::String(StringSchema::default()),
				}],
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
			.validate(&mut val!([{ "label": "buy milk" }]))
			.await
			.is_empty()
			.xpect_true();
		// a todo item missing its required `label` fails recursively
		resolved
			.validate(&mut val!([{}]))
			.await
			.is_empty()
			.xpect_false();
	}
}
