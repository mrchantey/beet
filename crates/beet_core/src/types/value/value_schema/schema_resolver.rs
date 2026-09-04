//! [`SchemaResolver`]: the registries a schema resolves its indirections against.
use crate::prelude::*;
use bevy_reflect::TypeRegistry;

/// The registries a [`ValueSchema`] or [`ValueSchema`] resolves against,
/// threaded through every resolution and validation seam.
///
/// [`SchemaRegistry`] is the one by-name namespace, authored and reflect-derived
/// schemas alike, plus the by-location index a schema document registers into.
/// Bevy's [`TypeRegistry`] is the reflect fallback a [`ValueSchema::TypePath`]
/// resolves through, and the only place the schema layer meets reflection: the
/// by-name registry answers first, so a hand-authored schema registered under a
/// type path wins over what reflection would derive for it.
///
/// Both borrows are optional, so [`SchemaResolver::default`] resolves nothing
/// and every indirection defers to [`ValueSchema::Any`], exactly as a schema
/// still arriving does.
#[derive(Default, Clone, Copy)]
pub struct SchemaResolver<'a> {
	schemas: Option<&'a SchemaRegistry>,
	types: Option<&'a TypeRegistry>,
}

impl core::fmt::Debug for SchemaResolver<'_> {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("SchemaResolver")
			.field("schemas", &self.schemas.map(SchemaRegistry::len))
			.field("types", &self.types.is_some())
			.finish()
	}
}

/// Bound on a chain of [`SchemaRef::Name`] hops, so a cyclic graph
/// (`A -> B -> A`) terminates at a deferred wildcard rather than looping.
const MAX_REFERENCE_HOPS: usize = 64;

impl<'a> SchemaResolver<'a> {
	/// A resolver over both registries.
	pub fn new(schemas: &'a SchemaRegistry, types: &'a TypeRegistry) -> Self {
		Self {
			schemas: Some(schemas),
			types: Some(types),
		}
	}

	/// Add the by-name schema registry.
	pub fn with_schemas(mut self, schemas: &'a SchemaRegistry) -> Self {
		self.schemas = Some(schemas);
		self
	}

	/// Add bevy's type registry, the reflect fallback.
	pub fn with_types(mut self, types: &'a TypeRegistry) -> Self {
		self.types = Some(types);
		self
	}

	/// The by-name registry, when this resolver has one.
	pub fn registry(&self) -> Option<&'a SchemaRegistry> { self.schemas }

	/// The schema registered under `name`, following a chain of references to
	/// the first schema that is not itself a reference.
	///
	/// `None` when the name is unregistered (still arriving, or never coming) or
	/// when the chain cycles. Self-recursion is *not* a cycle: a schema reaches
	/// its own reference only by descending into data, which is finite.
	pub fn schema(&self, name: &str) -> Option<&'a ValueSchema> {
		let registry = self.schemas?;
		let mut name = name;
		for _ in 0..MAX_REFERENCE_HOPS {
			match registry.get(name)? {
				ValueSchema::Ref(SchemaRef::Name(next)) => name = next.as_str(),
				schema => return Some(schema),
			}
		}
		None
	}

	/// Follow one [`SchemaRef`] to the schema it names, where the walk meets it.
	///
	/// `None` defers to a wildcard, which every arm does when it cannot answer:
	/// an unregistered or cyclic name, a schema document still arriving, and an
	/// [`AtField`](SchemaRef::AtField), which only the struct holding it can
	/// resolve and which is therefore bound before the walk descends.
	///
	/// A [`TypePath`](SchemaRef::TypePath) resolves through the by-name registry
	/// here rather than through reflection, because reflection builds an owned
	/// schema and this hop borrows. The reflect fallback lives at the
	/// declaration seam ([`ValueSchema::resolve`]), which returns one.
	pub fn follow(&self, schema_ref: &SchemaRef) -> Option<&'a ValueSchema> {
		match schema_ref {
			SchemaRef::Name(name) | SchemaRef::TypePath(name) => {
				self.schema(name)
			}
			SchemaRef::Document(path) => self.located(path),
			SchemaRef::AtField(_) => None,
		}
	}

	/// The schema of the schema document at `path`, once it has arrived.
	///
	/// Resolution by *location*: a second way in to the one stored schema, not
	/// a second copy of it.
	pub fn located(&self, path: &SmolPath) -> Option<&'a ValueSchema> {
		self.schemas?.located(path)
	}

	/// The schema of the registered Rust type at `path`.
	pub fn type_schema(&self, path: &str) -> Result<ValueSchema> {
		if let Some(schema) = self.schema(path) {
			return schema.clone().xok();
		}
		self.types
			.and_then(|types| types.get_with_type_path(path))
			.ok_or_else(|| bevyhow!("type `{path}` is not registered"))?
			.type_info()
			.xmap(ValueSchema::from_type_info)
			.xok()
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[derive(Reflect)]
	struct Count(#[allow(dead_code)] u32);

	#[crate::test]
	fn follows_a_reference_chain() {
		let mut registry = SchemaRegistry::default();
		registry.insert("A", ValueSchema::reference("B"));
		registry.insert("B", ValueSchema::of::<i64>());
		SchemaResolver::default()
			.with_schemas(&registry)
			.schema("A")
			.unwrap()
			.xpect_eq(ValueSchema::of::<i64>());
	}

	#[crate::test]
	fn a_cyclic_chain_defers() {
		let mut registry = SchemaRegistry::default();
		registry.insert("A", ValueSchema::reference("B"));
		registry.insert("B", ValueSchema::reference("A"));
		SchemaResolver::default()
			.with_schemas(&registry)
			.schema("A")
			.is_none()
			.xpect_true();
	}

	/// The by-name registry answers before reflection, which is how the
	/// hand-authored meta-schema stands in for the opaque `ValueSchema` type.
	#[crate::test]
	fn an_authored_schema_wins_over_reflection() {
		let mut types = bevy_reflect::TypeRegistry::default();
		types.register::<Count>();
		let mut registry = SchemaRegistry::default();
		registry.insert(Count::type_path(), ValueSchema::of::<i64>());
		SchemaResolver::new(&registry, &types)
			.type_schema(Count::type_path())
			.unwrap()
			.xpect_eq(ValueSchema::of::<i64>());
	}

	#[crate::test]
	fn an_unregistered_type_errors() {
		SchemaResolver::default()
			.type_schema("nope::Nope")
			.unwrap_err()
			.to_string()
			.xpect_contains("nope::Nope");
	}
}
