//! JSON Schema export from a [`ValueSchema`].
//!
//! The one reflection interpretation is [`ValueSchema::from_type_info`]; JSON
//! Schema is an *export* of what it produced, not a second walk over
//! [`TypeInfo`]. A composite that names itself is hoisted into a root `$defs`
//! section and referenced with `$ref`, which is also how a recursive schema's
//! [`SchemaRef::Name`] resolves.
//!
//! Export is fallible because a [`ValueSchema`] can name things JSON Schema
//! cannot: a registered Rust type, a schema document, or a sibling field. See
//! [`JsonSchema::try_from_schema`].
//!
//! # Example
//!
//! ```rust
//! use bevy::reflect::{Reflect, Typed};
//! use beet_core::prelude::*;
//!
//! #[derive(Reflect)]
//! struct MyRequest {
//!     name: String,
//!     count: u32,
//!     enabled: Option<bool>,
//! }
//!
//! // A JSON Schema object with properties, required fields, etc.
//! let schema = JsonSchema::new::<MyRequest>().unwrap();
//! ```
use crate::prelude::*;
use bevy::reflect::TypeInfo;
use bevy::reflect::Typed;

impl JsonSchema {
	/// Generates a schema for type `T`.
	pub fn new<T: Typed>() -> Result<Self> {
		Self::from_type_info(T::type_info())
	}

	/// Generates a schema from [`TypeInfo`], via the [`ValueSchema`] the type
	/// reflects to.
	pub fn from_type_info(type_info: &TypeInfo) -> Result<Self> {
		Self::try_from_schema(&ValueSchema::from_type_info(type_info))
	}

	/// Exports a [`ValueSchema`] as JSON Schema.
	///
	/// # Errors
	///
	/// A [`SchemaRef::TypePath`], [`SchemaRef::Document`] or
	/// [`SchemaRef::AtField`] is resolved at runtime against a registry or a
	/// sibling value and has no JSON Schema equivalent, as is a
	/// [`SchemaRef::Name`] naming nothing in this schema and a
	/// [`MapSchema::Keyed`] map, whose entries are typed by the registry. Rather than degrade
	/// them to an unconstrained `{}`, export says which reference it could not
	/// write.
	pub fn try_from_schema(schema: &ValueSchema) -> Result<Self> {
		let mut exporter = Exporter::default();
		exporter.index(schema);
		let mut root = exporter.inline(schema)?;
		if !exporter.defs.is_empty()
			&& let Ok(map) = root.as_map_mut()
		{
			map.insert("$defs", exporter.defs);
		}
		Self::from_value(root).xok()
	}
}

/// Writes the `Min | Max | Step` bounds every numeric constraint shares.
macro_rules! number_keywords {
	($map:expr, $constraints:expr, $constraint:ident) => {
		for constraint in $constraints {
			let (keyword, value, behavior) = match constraint {
				$constraint::Min(bound) => {
					("minimum", bound.value, bound.behavior)
				}
				$constraint::Max(bound) => {
					("maximum", bound.value, bound.behavior)
				}
				$constraint::Step(bound) => {
					("multipleOf", bound.value, bound.behavior)
				}
			};
			bound_keyword(&mut $map, keyword, value, behavior);
		}
	};
}

/// The export walk.
///
/// `named` is every composite in the schema that declares a name, so a
/// reference by name resolves without a registry; `defs` is the subset an
/// emitted `$ref` actually reached, and `building` is the set currently being
/// written into it, which is what stops a recursive schema recursing.
#[derive(Default)]
struct Exporter<'a> {
	named: HashMap<SmolStr, &'a ValueSchema>,
	defs: Map,
	building: HashSet<SmolStr>,
}

impl<'a> Exporter<'a> {
	/// Records every named composite in `schema`, outermost first, so a
	/// by-name reference to an ancestor still under construction resolves.
	fn index(&mut self, schema: &'a ValueSchema) {
		if let Some(name) = schema.name()
			&& !self.named.contains_key(name)
		{
			self.named.insert(name.clone(), schema);
		}
		match schema {
			ValueSchema::Struct(schema) => {
				for field in &schema.fields {
					self.index(&field.schema);
				}
			}
			ValueSchema::Tuple(schema) => {
				for field in &schema.fields {
					self.index(&field.schema);
				}
			}
			ValueSchema::List(schema) => self.index(&schema.item),
			ValueSchema::Map(MapSchema::Uniform { value }) => self.index(value),
			ValueSchema::Enum(schema) => {
				for payload in schema
					.variants
					.iter()
					.filter_map(|variant| variant.payload.as_ref())
				{
					self.index(payload);
				}
			}
			ValueSchema::Optional(schema) => self.index(schema),
			_ => {}
		}
	}

	/// A schema in a nested position: a composite that names itself becomes a
	/// `$defs` entry referenced by `$ref`, which keeps the emitted document flat
	/// and lets a repeated type be written once.
	fn nested(&mut self, schema: &'a ValueSchema) -> Result<Value> {
		match schema {
			ValueSchema::Ref(SchemaRef::Name(name)) => self.def_ref(name),
			ValueSchema::Ref(schema_ref) => bevybail!(
				"cannot export {schema_ref} as JSON Schema: it is resolved at runtime"
			),
			schema => match schema.name() {
				Some(name) => self.def_ref(name),
				None => self.inline(schema),
			},
		}
	}

	/// A `{"$ref": ..}` to `name`'s definition, writing the definition first if
	/// this is the reference that reached it.
	fn def_ref(&mut self, name: &SmolStr) -> Result<Value> {
		if !self.defs.contains_key(name) && self.building.insert(name.clone()) {
			let Some(schema) = self.named.get(name).copied() else {
				self.building.remove(name);
				bevybail!(
					"cannot export a reference to `{name}` as JSON Schema: no schema of that name is written here"
				);
			};
			let value = self.inline(schema)?;
			self.building.remove(name);
			self.defs.insert(name.clone(), value);
		}
		let mut reference = Map::default();
		reference.insert("$ref", Value::str(format!("#/$defs/{name}")));
		Value::Map(reference).xok()
	}

	/// The schema written out in place, whatever it names itself.
	fn inline(&mut self, schema: &'a ValueSchema) -> Result<Value> {
		let mut map = match schema {
			// the unconstrained schema: JSON Schema spells "anything" as an
			// empty object, not as a type.
			ValueSchema::Any => Map::default(),
			ValueSchema::Null => keyword("null"),
			ValueSchema::Bool(_) => keyword("boolean"),
			ValueSchema::I64(schema) => {
				let mut map = keyword("integer");
				number_keywords!(map, &schema.constraints, I64Constraint);
				map
			}
			ValueSchema::U64(schema) => {
				let mut map = keyword("integer");
				number_keywords!(map, &schema.constraints, U64Constraint);
				map
			}
			ValueSchema::F64(schema) => {
				let mut map = keyword("number");
				number_keywords!(map, &schema.constraints, F64Constraint);
				map
			}
			ValueSchema::String(schema) => string_schema(schema),
			// bytes encode as an array of octets, the way `Value::Bytes` does.
			ValueSchema::Bytes(schema) => {
				let mut octet = keyword("integer");
				octet.insert("minimum", 0u64);
				octet.insert("maximum", 255u64);
				let mut map = keyword("array");
				map.insert("items", Value::Map(octet));
				if let Some(max_len) = schema.max_len {
					map.insert("maxItems", max_len as u64);
				}
				map
			}
			// a node key rather than a number, but it travels as one.
			ValueSchema::Entity(_) => {
				let mut map = keyword("integer");
				map.insert("minimum", 0u64);
				map
			}
			ValueSchema::Struct(schema) => {
				let mut properties = Map::default();
				let mut required = Vec::new();
				for field in &schema.fields {
					let value = self.nested(&field.schema)?;
					if field.required {
						required.push(Value::str(field.key.as_str()));
					}
					properties.insert(
						field.key.clone(),
						annotated(
							value,
							field.description.as_ref(),
							default_of(field),
						),
					);
				}
				let mut map = keyword("object");
				map.insert("properties", properties);
				if !schema.allow_additional {
					map.insert("additionalProperties", false);
				}
				if !required.is_empty() {
					map.insert("required", required);
				}
				map
			}
			ValueSchema::Tuple(schema) => {
				let mut prefix_items = Vec::new();
				for field in &schema.fields {
					let value = self.nested(&field.schema)?;
					prefix_items.push(annotated(
						value,
						field.description.as_ref(),
						None,
					));
				}
				let mut map = keyword("array");
				map.insert("prefixItems", prefix_items);
				map.insert("items", false);
				map
			}
			ValueSchema::List(schema) => {
				let item = self.nested(&schema.item)?;
				let mut map = keyword("array");
				map.insert("items", item);
				if let Some(min_items) = schema.min_items {
					map.insert("minItems", min_items as u64);
				}
				if let Some(max_items) = schema.max_items {
					map.insert("maxItems", max_items as u64);
				}
				if schema.unique {
					map.insert("uniqueItems", true);
				}
				map
			}
			ValueSchema::Map(MapSchema::Keyed) => bevybail!(
				"cannot export a keyed map as JSON Schema: its entries are resolved at runtime"
			),
			ValueSchema::Map(MapSchema::Uniform { value }) => {
				let value = self.nested(value)?;
				let mut map = keyword("object");
				map.insert("additionalProperties", value);
				map
			}
			ValueSchema::Enum(schema) => self.enum_schema(schema)?,
			ValueSchema::Optional(schema) => {
				let inner = self.nested(schema)?;
				let mut map = Map::default();
				map.insert("oneOf", vec![Value::Map(keyword("null")), inner]);
				map
			}
			ValueSchema::Ref(_) => return self.nested(schema),
		};
		if let Some(name) = schema.name() {
			map.insert("title", name.as_str());
		}
		if let Some(description) = schema.description() {
			map.insert("description", description.as_str());
		}
		Value::Map(map).xok()
	}

	/// An enum of only unit variants is a string of variant names; anything else
	/// is a `oneOf` over the externally tagged form each variant serializes as.
	fn enum_schema(&mut self, schema: &'a EnumSchema) -> Result<Map> {
		if schema
			.variants
			.iter()
			.all(|variant| variant.payload.is_none())
		{
			let names = schema
				.variants
				.iter()
				.map(|variant| Value::str(variant.name.as_str()))
				.collect::<Vec<_>>();
			let mut map = keyword("string");
			map.insert("enum", names);
			return map.xok();
		}
		let mut one_of = Vec::new();
		for variant in &schema.variants {
			one_of.push(Value::Map(match &variant.payload {
				None => {
					let mut map = Map::default();
					map.insert("const", Value::str(variant.name.as_str()));
					map
				}
				Some(payload) => {
					let mut properties = Map::default();
					properties
						.insert(variant.name.clone(), self.nested(payload)?);
					let mut map = keyword("object");
					map.insert("properties", properties);
					map.insert("required", vec![Value::str(
						variant.name.as_str(),
					)]);
					map.insert("additionalProperties", false);
					map
				}
			}));
		}
		let mut map = Map::default();
		map.insert("oneOf", one_of);
		map.xok()
	}
}

/// A `{"type": ..}` map, the start of most schemas.
fn keyword(type_name: &str) -> Map {
	let mut map = Map::default();
	map.insert("type", type_name);
	map
}

/// Writes a bound as a JSON Schema keyword, but only where it *rejects*: a
/// [`ConstraintBehavior::Mutate`] bound coerces the value instead, so emitting
/// it would claim JSON validation rejects an input the runtime accepts.
fn bound_keyword(
	map: &mut Map,
	keyword: &str,
	value: impl Into<Value>,
	behavior: ConstraintBehavior,
) {
	if let ConstraintBehavior::Error = behavior {
		map.insert(keyword, value.into());
	}
}

/// `multiline` has no JSON Schema equivalent and is not written.
fn string_schema(schema: &StringSchema) -> Map {
	let mut map = keyword("string");
	for constraint in &schema.constraints {
		match constraint {
			StringConstraint::MinLength { value, behavior } => {
				bound_keyword(&mut map, "minLength", *value as u64, *behavior)
			}
			StringConstraint::MaxLength { value, behavior } => {
				bound_keyword(&mut map, "maxLength", *value as u64, *behavior)
			}
			StringConstraint::Email => {
				map.insert("format", "email");
			}
		}
	}
	if schema.sensitive {
		map.insert("writeOnly", true);
	}
	map
}

/// The value a commit assigns when the field is absent, the only
/// [`OnMissing`] arm JSON Schema can state. `Error` is the absence of a default
/// and `Computed` runs a script, neither of which is a `default` keyword.
fn default_of(field: &NamedFieldSchema) -> Option<&Value> {
	match &field.on_missing {
		Some(OnMissing::Default(value)) => Some(value),
		_ => None,
	}
}

/// A field's annotations written onto its schema. A `$ref` may carry no sibling
/// keywords in strict mode, so an annotated reference is wrapped in a one-arm
/// `anyOf`.
fn annotated(
	mut schema: Value,
	description: Option<&SmolStr>,
	default: Option<&Value>,
) -> Value {
	let mut annotations = Map::default();
	if let Some(description) = description {
		annotations.insert("description", description.as_str());
	}
	if let Some(default) = default {
		annotations.insert("default", default.clone());
	}
	if annotations.is_empty() {
		return schema;
	}
	if schema.get("$ref").is_some() {
		annotations.insert("anyOf", vec![schema]);
		return Value::Map(annotations);
	}
	if let Ok(map) = schema.as_map_mut() {
		for (key, value) in annotations {
			map.insert(key, value);
		}
	}
	schema
}

#[cfg(test)]
mod test {
	use super::*;

	/// A struct of the three scalar kinds.
	#[derive(Reflect)]
	struct SimpleStruct {
		name: String,
		count: u32,
		enabled: bool,
	}

	#[derive(Reflect)]
	struct WithOptional {
		required_field: String,
		optional_field: Option<String>,
	}

	#[derive(Reflect)]
	struct WithNested {
		inner: SimpleStruct,
		value: i64,
	}

	#[derive(Reflect)]
	struct WithVec {
		items: Vec<String>,
		numbers: Vec<i32>,
	}

	#[derive(Reflect)]
	enum SimpleEnum {
		First,
		Second,
		Third,
	}

	#[derive(Reflect)]
	enum ComplexEnum {
		Unit,
		Tuple(String),
		Struct { x: f32, y: f32 },
	}

	#[derive(Reflect)]
	struct TupleStruct(String, i32);

	#[derive(Reflect)]
	struct NewtypeStruct(String);

	#[derive(Reflect)]
	struct ComplexStruct {
		simple_enum: SimpleEnum,
		complex_enum: Option<ComplexEnum>,
		field: bool,
	}

	/// The value at a `/` separated path of map keys.
	fn at<'a>(value: &'a Value, path: &str) -> &'a Value {
		path.split('/').fold(value, |value, key| {
			value
				.get(key)
				.unwrap_or_else(|| panic!("no `{key}` in {value:?}"))
		})
	}

	/// The string at a `/` separated path of map keys.
	fn string_at(value: &Value, path: &str) -> String {
		at(value, path).as_str().unwrap().to_string()
	}

	fn schema_of<T: Typed>() -> JsonSchema { JsonSchema::new::<T>().unwrap() }

	#[crate::test]
	fn set_field_enum_constrains_a_string_field() {
		// a plain string field gains a runtime `enum` constraint at a top-level property.
		let mut schema = schema_of::<SimpleStruct>();
		schema.set_field_enum("name", ["red", "green"].map(SmolStr::from));
		at(&schema, "properties/name/enum")
			.clone()
			.xpect_eq(Value::List(vec![
				Value::str("red"),
				Value::str("green"),
			]));
	}

	#[crate::test]
	fn simple_struct_schema() {
		let schema = schema_of::<SimpleStruct>();
		string_at(&schema, "type").xpect_eq("object");
		string_at(&schema, "title").xpect_eq("SimpleStruct");
		string_at(&schema, "properties/name/type").xpect_eq("string");
		string_at(&schema, "properties/count/type").xpect_eq("integer");
		string_at(&schema, "properties/enabled/type").xpect_eq("boolean");
		at(&schema, "required").as_list().unwrap().len().xpect_eq(3);
	}

	#[crate::test]
	fn complex_struct_schema() {
		let schema = schema_of::<ComplexStruct>();
		string_at(&schema, "$defs/SimpleEnum/type").xpect_eq("string");
		at(&schema, "$defs/SimpleEnum/enum")
			.as_list()
			.unwrap()
			.len()
			.xpect_eq(3);
		at(&schema, "$defs/ComplexEnum/oneOf")
			.as_list()
			.unwrap()
			.len()
			.xpect_eq(3);
		string_at(&schema, "properties/simple_enum/$ref")
			.xpect_eq("#/$defs/SimpleEnum");

		// an optional enum is a null-or-reference union, and the reference is
		// still the shared definition
		let one_of = at(&schema, "properties/complex_enum/oneOf")
			.as_list()
			.unwrap();
		one_of.len().xpect_eq(2);
		string_at(&one_of[0], "type").xpect_eq("null");
		string_at(&one_of[1], "$ref").xpect_eq("#/$defs/ComplexEnum");
		string_at(&schema, "properties/field/type").xpect_eq("boolean");
	}

	#[crate::test]
	fn nested_struct_schema() {
		let schema = schema_of::<WithNested>();
		string_at(&schema, "$defs/SimpleStruct/type").xpect_eq("object");
		string_at(&schema, "$defs/SimpleStruct/title").xpect_eq("SimpleStruct");
		string_at(&schema, "$defs/SimpleStruct/properties/name/type")
			.xpect_eq("string");
		string_at(&schema, "properties/inner/$ref")
			.xpect_eq("#/$defs/SimpleStruct");
		string_at(&schema, "properties/value/type").xpect_eq("integer");
	}

	#[crate::test]
	fn optional_fields_not_required() {
		let schema = schema_of::<WithOptional>();
		let required = at(&schema, "required").as_list().unwrap();
		required.len().xpect_eq(1);
		required[0].as_str().unwrap().xpect_eq("required_field");
		at(&schema, "properties/optional_field")
			.get("oneOf")
			.is_some()
			.xpect_true();
	}

	#[crate::test]
	fn vec_field_schema() {
		let schema = schema_of::<WithVec>();
		string_at(&schema, "properties/items/type").xpect_eq("array");
		string_at(&schema, "properties/items/items/type").xpect_eq("string");
		string_at(&schema, "properties/numbers/items/type").xpect_eq("integer");
	}

	#[crate::test]
	fn simple_enum_schema() {
		let schema = schema_of::<SimpleEnum>();
		string_at(&schema, "type").xpect_eq("string");
		at(&schema, "enum")
			.as_list()
			.unwrap()
			.iter()
			.map(|variant| variant.as_str().unwrap())
			.collect::<Vec<_>>()
			.xpect_eq(vec!["First", "Second", "Third"]);
	}

	/// A mixed enum is a union of the externally tagged forms its variants
	/// serialize as: a bare name for a unit variant, a one-key object otherwise.
	#[crate::test]
	fn complex_enum_schema() {
		let schema = schema_of::<ComplexEnum>();
		let one_of = at(&schema, "oneOf").as_list().unwrap();
		one_of.len().xpect_eq(3);
		string_at(&one_of[0], "const").xpect_eq("Unit");
		string_at(&one_of[1], "properties/Tuple/type").xpect_eq("string");
		string_at(&one_of[2], "properties/Struct/properties/x/type")
			.xpect_eq("number");
		one_of[1]
			.get("required")
			.unwrap()
			.as_list()
			.unwrap()
			.contains(&Value::str("Tuple"))
			.xpect_true();
	}

	#[crate::test]
	fn tuple_struct_schema() {
		let schema = schema_of::<TupleStruct>();
		string_at(&schema, "type").xpect_eq("array");
		at(&schema, "prefixItems")
			.as_list()
			.unwrap()
			.len()
			.xpect_eq(2);
		at(&schema, "items").clone().xpect_eq(Value::Bool(false));
	}

	#[crate::test]
	fn newtype_struct_unwraps() {
		string_at(&schema_of::<NewtypeStruct>(), "type").xpect_eq("string");
	}

	#[crate::test]
	fn unit_type_schema() {
		string_at(&schema_of::<()>(), "type").xpect_eq("null");
	}

	/// A `SmolPath` newtype field unwraps to its inner `SmolStr` and must lower
	/// to a `string` property, mirroring the blob-store action params. An empty
	/// object schema here is rejected by OpenAI strict mode.
	#[crate::test]
	fn string_newtype_field_is_string() {
		#[derive(Reflect)]
		struct WithPath {
			path: SmolPath,
		}
		string_at(&schema_of::<WithPath>(), "properties/path/type")
			.xpect_eq("string");
	}

	/// A `Duration` field lowers to a `string` property (authored `"1.5s"`), not the
	/// `object` fallthrough — the empty object broke the perceive-act tool schema under
	/// OpenAI strict mode (nested `DriveForDuration { duration: Duration }`).
	#[crate::test]
	fn duration_field_is_string() {
		#[derive(Reflect)]
		struct WithDuration {
			duration: core::time::Duration,
		}
		string_at(&schema_of::<WithDuration>(), "properties/duration/type")
			.xpect_eq("string");
	}

	/// A recursive type's by-name reference resolves against the definition the
	/// export hoists out of the same schema, rather than dangling.
	#[crate::test]
	fn a_recursive_type_references_its_own_definition() {
		#[derive(Reflect)]
		struct Node {
			label: String,
			children: Vec<Node>,
		}
		let schema = schema_of::<Node>();
		string_at(&schema, "properties/children/items/$ref")
			.xpect_eq("#/$defs/Node");
		string_at(&schema, "$defs/Node/properties/children/items/$ref")
			.xpect_eq("#/$defs/Node");
		string_at(&schema, "$defs/Node/properties/label/type")
			.xpect_eq("string");
	}

	/// Doc comments survive the trip through [`ValueSchema`]: a type's own on the
	/// schema it names, a field's on the field, and a field naming a definition
	/// wraps it, since a `$ref` carries no siblings.
	#[cfg(feature = "bevy_reflect_documentation")]
	#[crate::test]
	fn documentation_is_exported() {
		/// A documented leaf.
		#[derive(Reflect)]
		struct Leaf {
			/// The leaf's own label.
			label: String,
		}
		/// A documented root.
		#[derive(Reflect)]
		struct Root {
			/// The leaf this root holds.
			leaf: Leaf,
		}
		let schema = schema_of::<Root>();
		string_at(&schema, "description").xpect_eq(" A documented root.");
		string_at(&schema, "$defs/Leaf/description")
			.xpect_eq(" A documented leaf.");
		string_at(&schema, "$defs/Leaf/properties/label/description")
			.xpect_eq(" The leaf's own label.");
		string_at(&schema, "properties/leaf/description")
			.xpect_eq(" The leaf this root holds.");
		string_at(
			&at(&schema, "properties/leaf/anyOf").as_list().unwrap()[0],
			"$ref",
		)
		.xpect_eq("#/$defs/Leaf");
	}

	/// An entity field is a node key, exported as a non-negative integer rather
	/// than the opaque fallthrough.
	#[crate::test]
	fn an_entity_field_is_a_node_key() {
		#[derive(Reflect)]
		struct Link {
			target: Entity,
		}
		let schema = schema_of::<Link>();
		string_at(&schema, "properties/target/type").xpect_eq("integer");
		at(&schema, "properties/target/minimum")
			.clone()
			.xpect_eq(Value::Uint(0));
	}

	/// Nothing is known about an unregistered opaque type, which JSON Schema
	/// spells as the empty schema. The previous `{"type":"object"}` claimed a
	/// shape the value does not have.
	#[crate::test]
	fn an_unknown_opaque_type_is_unconstrained() {
		JsonSchema::try_from_schema(&ValueSchema::Any)
			.unwrap()
			.into_inner()
			.xpect_eq(Value::Map(Map::default()));
	}

	/// A rejecting bound is a JSON Schema keyword; a coercing one is not, since
	/// the runtime accepts the value it would reject.
	#[crate::test]
	fn only_rejecting_constraints_export() {
		let rejects =
			JsonSchema::try_from_schema(&ValueSchema::U64(U64Schema {
				constraints: vec![U64Constraint::Max(U64Max {
					value: 10,
					behavior: ConstraintBehavior::Error,
				})],
			}))
			.unwrap();
		at(&rejects, "maximum").clone().xpect_eq(Value::Uint(10));

		let coerces =
			JsonSchema::try_from_schema(&ValueSchema::U64(U64Schema {
				constraints: vec![U64Constraint::Max(U64Max {
					value: 10,
					behavior: ConstraintBehavior::Mutate,
				})],
			}))
			.unwrap();
		coerces.get("maximum").xpect_none();
	}

	/// Bytes travel as the octet array `Value::Bytes` encodes as.
	#[crate::test]
	fn bytes_are_an_octet_array() {
		let schema =
			JsonSchema::try_from_schema(&ValueSchema::Bytes(BytesSchema {
				max_len: Some(4),
			}))
			.unwrap();
		string_at(&schema, "type").xpect_eq("array");
		string_at(&schema, "items/type").xpect_eq("integer");
		at(&schema, "maxItems").clone().xpect_eq(Value::Uint(4));
	}

	/// A field whose commit policy assigns a value states it as the `default`
	/// keyword. `Error` and `Computed` have no such value to state.
	#[crate::test]
	fn a_field_default_is_exported() {
		let schema = ValueSchema::Struct(StructSchema {
			name: None,
			description: None,
			allow_additional: false,
			fields: vec![
				NamedFieldSchema::new("tries", ValueSchema::U64(default()))
					.with_on_missing(OnMissing::Default(Value::Uint(3))),
				NamedFieldSchema::new("name", ValueSchema::String(default()))
					.with_on_missing(OnMissing::Error),
			],
		});
		let schema = JsonSchema::try_from_schema(&schema).unwrap();
		at(&schema, "properties/tries/default")
			.clone()
			.xpect_eq(Value::Uint(3));
		at(&schema, "properties/name").get("default").xpect_none();
	}

	/// A reference resolved at runtime has no JSON Schema equivalent, and export
	/// says so rather than writing an unconstrained schema.
	#[crate::test]
	fn a_runtime_reference_cannot_be_exported() {
		JsonSchema::try_from_schema(&ValueSchema::at_field("kind"))
			.unwrap_err()
			.to_string()
			.xpect_contains("the schema at `kind`");
		JsonSchema::try_from_schema(&ValueSchema::reference("Missing"))
			.unwrap_err()
			.to_string()
			.xpect_contains("Missing");
	}
}
