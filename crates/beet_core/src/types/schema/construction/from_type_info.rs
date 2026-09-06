//! Conversion from bevy reflect [`TypeInfo`] to [`ValueSchema`].
//!
//! Mirrors [`crate::types::json_schema`] but produces a [`ValueSchema`] suitable
//! for validation and UI generation.
use crate::prelude::*;
use bevy::reflect::NamedField;
use bevy::reflect::TypeInfo;
use bevy::reflect::Typed;
use bevy::reflect::UnnamedField;
use bevy::reflect::array::ArrayInfo;
use bevy::reflect::enums::EnumInfo;
use bevy::reflect::enums::VariantInfo;
use bevy::reflect::list::ListInfo;
use bevy::reflect::map::MapInfo;
use bevy::reflect::set::SetInfo;
use bevy::reflect::structs::StructInfo;
use bevy::reflect::tuple::TupleInfo;
use bevy::reflect::tuple_struct::TupleStructInfo;

impl ValueSchema {
	/// Build a schema for `T` via its bevy reflect type info.
	pub fn of<T: Typed>() -> Self { Self::from_type_info(T::type_info()) }

	/// Build a schema from a bevy reflect [`TypeInfo`].
	pub fn from_type_info(type_info: &TypeInfo) -> Self {
		Builder::default().build(type_info)
	}
}

/// The recursive walk, tracking named ancestor types so a self-referential
/// type (eg `SidebarNode { children: Vec<SidebarNode> }`) lowers to a
/// [`SchemaRef::Name`] by short type path instead of recursing forever.
#[derive(Default)]
struct Builder {
	visiting: Vec<SmolStr>,
}

impl Builder {
	fn build(&mut self, type_info: &TypeInfo) -> ValueSchema {
		// only named types (struct/tuple-struct/enum) can cycle
		let named = matches!(
			type_info,
			TypeInfo::Struct(_) | TypeInfo::TupleStruct(_) | TypeInfo::Enum(_)
		);
		let path = SmolStr::from(type_info.type_path());
		if named {
			if self.visiting.contains(&path) {
				return ValueSchema::Ref(SchemaRef::Name(SmolStr::from(
					type_info.type_path_table().short_path(),
				)));
			}
			self.visiting.push(path);
		}
		let schema = match type_info {
			TypeInfo::Struct(info) => {
				ValueSchema::Struct(self.struct_schema(info))
			}
			TypeInfo::TupleStruct(info) => self.tuple_struct_schema(info),
			TypeInfo::Tuple(info) => {
				if info.field_len() == 0 {
					ValueSchema::Null
				} else {
					ValueSchema::Tuple(self.tuple_schema(info, None))
				}
			}
			TypeInfo::List(info) => ValueSchema::List(self.list_schema(info)),
			TypeInfo::Array(info) => ValueSchema::List(self.array_schema(info)),
			TypeInfo::Map(info) => ValueSchema::Map(self.map_schema(info)),
			TypeInfo::Set(info) => ValueSchema::List(set_schema(info)),
			TypeInfo::Enum(info) => self.enum_schema(info),
			TypeInfo::Opaque(info) => primitive_schema(info.type_path()),
		};
		if named {
			self.visiting.pop();
		}
		schema
	}

	fn resolve_field(
		&mut self,
		type_info: Option<&TypeInfo>,
		type_path: &str,
	) -> ValueSchema {
		match type_info {
			Some(info) => self.build(info),
			None => primitive_schema(type_path),
		}
	}

	fn struct_schema(&mut self, info: &StructInfo) -> StructSchema {
		let fields = info
			.iter()
			.map(|field| self.named_field_schema(field))
			.collect();
		StructSchema {
			name: Some(SmolStr::from(info.type_path_table().short_path())),
			allow_additional: false,
			fields,
		}
	}

	fn named_field_schema(&mut self, field: &NamedField) -> NamedFieldSchema {
		let required = is_required_field(field.type_path());
		let schema = self.resolve_field(field.type_info(), field.type_path());

		#[cfg(feature = "bevy_reflect_documentation")]
		let description = field.docs().map(SmolStr::from);
		#[cfg(not(feature = "bevy_reflect_documentation"))]
		let description = None;

		NamedFieldSchema {
			key: SmolStr::from(field.name()),
			required,
			label: None,
			description,
			on_missing: None,
			schema,
		}
	}

	fn unnamed_field_schema(
		&mut self,
		field: &UnnamedField,
	) -> UnnamedFieldSchema {
		let required = is_required_field(field.type_path());
		let schema = self.resolve_field(field.type_info(), field.type_path());

		#[cfg(feature = "bevy_reflect_documentation")]
		let description = field.docs().map(SmolStr::from);
		#[cfg(not(feature = "bevy_reflect_documentation"))]
		let description = None;

		UnnamedFieldSchema {
			required,
			description,
			schema,
		}
	}

	fn tuple_struct_schema(&mut self, info: &TupleStructInfo) -> ValueSchema {
		// Newtypes unwrap to their inner type.
		if info.field_len() == 1 {
			let field = info.field_at(0).expect("tuple struct has 1 field");
			return self.resolve_field(field.type_info(), field.type_path());
		}
		let fields = info
			.iter()
			.map(|field| self.unnamed_field_schema(field))
			.collect();
		ValueSchema::Tuple(TupleSchema {
			name: Some(SmolStr::from(info.type_path_table().short_path())),
			fields,
		})
	}

	fn tuple_schema(
		&mut self,
		info: &TupleInfo,
		name: Option<SmolStr>,
	) -> TupleSchema {
		let fields = info
			.iter()
			.map(|field| self.unnamed_field_schema(field))
			.collect();
		TupleSchema { name, fields }
	}

	fn list_schema(&mut self, info: &ListInfo) -> ListSchema {
		let item = self.resolve_field(info.item_info(), info.item_ty().path());
		ListSchema {
			item: Box::new(item),
			min_items: None,
			max_items: None,
			unique: false,
		}
	}

	fn array_schema(&mut self, info: &ArrayInfo) -> ListSchema {
		let item = self.resolve_field(info.item_info(), info.item_ty().path());
		ListSchema {
			item: Box::new(item),
			min_items: Some(info.capacity()),
			max_items: Some(info.capacity()),
			unique: false,
		}
	}

	fn map_schema(&mut self, info: &MapInfo) -> MapSchema {
		let value =
			self.resolve_field(info.value_info(), info.value_ty().path());
		MapSchema {
			value: Box::new(value),
		}
	}

	fn enum_schema(&mut self, info: &EnumInfo) -> ValueSchema {
		// Treat `Option<T>` specially: an optional wrapper over the inner schema, so a
		// null or missing value validates while a present value is typed as `T`.
		if is_option_type(info.type_path())
			&& let Some(VariantInfo::Tuple(some_info)) = info.variant("Some")
			&& let Some(field) = some_info.field_at(0)
		{
			return ValueSchema::Optional(Box::new(
				self.resolve_field(field.type_info(), field.type_path()),
			));
		}

		let variants =
			info.iter()
				.map(|variant| match variant {
					VariantInfo::Unit(variant) => VariantSchema {
						name: SmolStr::from(variant.name()),
						payload: None,
					},
					VariantInfo::Tuple(variant) => {
						if variant.field_len() == 1 {
							let field = variant.field_at(0).expect("len == 1");
							VariantSchema {
								name: SmolStr::from(variant.name()),
								payload: Some(self.resolve_field(
									field.type_info(),
									field.type_path(),
								)),
							}
						} else {
							let fields = variant
								.iter()
								.map(|field| self.unnamed_field_schema(field))
								.collect();
							VariantSchema {
								name: SmolStr::from(variant.name()),
								payload: Some(ValueSchema::Tuple(
									TupleSchema { name: None, fields },
								)),
							}
						}
					}
					VariantInfo::Struct(variant) => {
						let fields = variant
							.iter()
							.map(|field| self.named_field_schema(field))
							.collect();
						VariantSchema {
							name: SmolStr::from(variant.name()),
							payload: Some(ValueSchema::Struct(StructSchema {
								name: None,
								allow_additional: false,
								fields,
							})),
						}
					}
				})
				.collect();
		ValueSchema::Enum(EnumSchema {
			name: Some(SmolStr::from(info.type_path_table().short_path())),
			variants,
		})
	}
}

fn set_schema(info: &SetInfo) -> ListSchema {
	let item = primitive_schema(info.value_ty().path());
	ListSchema {
		item: Box::new(item),
		min_items: None,
		max_items: None,
		unique: true,
	}
}

fn primitive_schema(type_path: &str) -> ValueSchema {
	if is_option_type(type_path) {
		// Outer Option<T> path with no resolvable inner type info: an optional
		// wrapper over the inner primitive (best effort).
		if let Some(inner) = extract_option_inner(type_path) {
			return ValueSchema::Optional(Box::new(primitive_schema(inner)));
		}
	}
	let short = type_path.rsplit("::").next().unwrap_or(type_path);
	match short {
		// `Duration` reflects as opaque but is authored as a unit-suffixed string
		// (eg `"30s"`), coerced by `scalar_to_reflect`, so validate it as a string.
		"String" | "str" | "char" | "PathBuf" | "OsString" | "SmolStr"
		| "SmolPath" | "Duration" => ValueSchema::String(StringSchema::default()),
		"u8" | "u16" | "u32" | "u64" | "u128" | "usize" => {
			ValueSchema::U64(U64Schema::default())
		}
		"i8" | "i16" | "i32" | "i64" | "i128" | "isize" => {
			ValueSchema::I64(I64Schema::default())
		}
		"f32" | "f64" => ValueSchema::F64(F64Schema::default()),
		"bool" => ValueSchema::Bool(BoolSchema::default()),
		// a node reference, not a number: its own kind so it remaps through the
		// entity map and a UI renders a picker.
		"Entity" => ValueSchema::Entity(EntitySchema::default()),
		"()" => ValueSchema::Null,
		_ => ValueSchema::Null,
	}
}

fn is_required_field(type_path: &str) -> bool {
	!type_path.starts_with("core::option::Option<")
		&& !type_path.starts_with("Option<")
}

fn is_option_type(type_path: &str) -> bool {
	type_path.starts_with("core::option::Option<")
		|| type_path.starts_with("Option<")
}

fn extract_option_inner(type_path: &str) -> Option<&str> {
	let path = type_path.trim();
	let inner = if let Some(rest) = path.strip_prefix("core::option::Option<") {
		Some(rest)
	} else {
		path.strip_prefix("Option<")
	}?;
	inner.strip_suffix('>')
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[derive(Reflect)]
	#[allow(dead_code)]
	struct UserProfile {
		name: String,
		age: u32,
		email: Option<String>,
	}

	#[derive(Reflect)]
	#[allow(dead_code)]
	enum Status {
		Active,
		Banned,
		Pending(String),
	}

	/// A self-referential tree type, the schema cycle case.
	#[derive(Default, Reflect)]
	struct Node {
		#[allow(dead_code)]
		label: String,
		#[allow(dead_code)]
		children: Vec<Node>,
	}

	#[crate::test]
	fn primitive_schemas() {
		matches!(ValueSchema::of::<bool>(), ValueSchema::Bool(_)).xpect_true();
		matches!(ValueSchema::of::<i32>(), ValueSchema::I64(_)).xpect_true();
		matches!(ValueSchema::of::<u32>(), ValueSchema::U64(_)).xpect_true();
		matches!(ValueSchema::of::<f32>(), ValueSchema::F64(_)).xpect_true();
		matches!(ValueSchema::of::<String>(), ValueSchema::String(_))
			.xpect_true();
		matches!(ValueSchema::of::<()>(), ValueSchema::Null).xpect_true();
	}

	#[crate::test]
	fn struct_schema_from_type_info() {
		let schema = ValueSchema::of::<UserProfile>();
		let ValueSchema::Struct(schema) = schema else {
			panic!("expected struct schema");
		};
		schema.fields.len().xpect_eq(3);
		schema.fields[0].key.as_str().xpect_eq("name");
		schema.fields[0].required.xpect_true();
		// Option<String> is represented by an optional, non-required field.
		schema.fields[2].key.as_str().xpect_eq("email");
		schema.fields[2].required.xpect_false();
	}

	#[crate::test]
	fn enum_schema_from_type_info() {
		let schema = ValueSchema::of::<Status>();
		let ValueSchema::Enum(schema) = schema else {
			panic!("expected enum schema");
		};
		schema.variants.len().xpect_eq(3);
		schema.variants[0].name.as_str().xpect_eq("Active");
		schema.variants[0].payload.is_none().xpect_true();
		schema.variants[2].name.as_str().xpect_eq("Pending");
		schema.variants[2].payload.is_some().xpect_true();
	}

	#[crate::test]
	fn optional_schema_built_for_option_field() {
		let schema = ValueSchema::of::<UserProfile>();
		let ValueSchema::Struct(struct_schema) = schema else {
			panic!("expected struct schema");
		};
		// `email: Option<String>` is an Optional wrapper over String.
		let email = &struct_schema.fields[2];
		matches!(email.schema, ValueSchema::Optional(_)).xpect_true();
	}

	/// A struct holding an entity reference lowers the field to the entity kind,
	/// so a form generated from the type knows to render a picker.
	#[crate::test]
	fn entity_field_lowers_to_the_entity_kind() {
		#[derive(Reflect)]
		#[allow(dead_code)]
		struct Link {
			target: Entity,
		}
		let ValueSchema::Struct(schema) = ValueSchema::of::<Link>() else {
			panic!("expected struct schema");
		};
		schema.fields[0]
			.schema
			.clone()
			.xpect_eq(ValueSchema::Entity(default()));
	}

	#[crate::test]
	fn recursive_type_lowers_to_reference() {
		let ValueSchema::Struct(schema) = ValueSchema::of::<Node>() else {
			panic!("expected struct schema");
		};
		// the recursive `children` list item is a by-name reference, not a cycle
		let children = schema
			.fields
			.iter()
			.find(|field| field.key == "children")
			.unwrap();
		let ValueSchema::List(list) = &children.schema else {
			panic!("expected list schema");
		};
		list.item.as_ref().xpect_eq(ValueSchema::reference("Node"));
	}
}
