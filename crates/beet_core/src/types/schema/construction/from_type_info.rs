//! Conversion from bevy reflect [`TypeInfo`] to [`ValueSchema`].
//!
//! The one interpretation of a reflected Rust type: validation, form generation
//! and the [`JsonSchema`] export in [`crate::types::json_schema`] all read what
//! this walk produced, rather than walking [`TypeInfo`] again themselves.
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
			TypeInfo::Struct(info) => ValueSchema::Struct(
				self.struct_schema(info, type_docs(type_info)),
			),
			TypeInfo::TupleStruct(info) => {
				self.tuple_struct_schema(info, type_docs(type_info))
			}
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
			TypeInfo::Enum(info) => {
				self.enum_schema(info, type_docs(type_info))
			}
			TypeInfo::Opaque(info) => type_path_schema(info.type_path()),
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
			None => type_path_schema(type_path),
		}
	}

	fn struct_schema(
		&mut self,
		info: &StructInfo,
		description: Option<SmolStr>,
	) -> StructSchema {
		let fields = info
			.iter()
			.map(|field| self.named_field_schema(field))
			.collect();
		StructSchema {
			name: Some(SmolStr::from(info.type_path_table().short_path())),
			description,
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

	fn tuple_struct_schema(
		&mut self,
		info: &TupleStructInfo,
		description: Option<SmolStr>,
	) -> ValueSchema {
		// Newtypes unwrap to their inner type; the wrapper's own name and docs
		// go with the wrapper.
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
			description,
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
		TupleSchema {
			name,
			description: None,
			fields,
		}
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

	fn enum_schema(
		&mut self,
		info: &EnumInfo,
		description: Option<SmolStr>,
	) -> ValueSchema {
		// Treat `Option<T>` specially: an optional wrapper over the inner schema, so a
		// null or missing value validates while a present value is typed as `T`.
		if matches!(generic_parts(info.type_path()), Some(("Option", _)))
			&& let Some(VariantInfo::Tuple(some_info)) = info.variant("Some")
			&& let Some(field) = some_info.field_at(0)
		{
			return ValueSchema::Optional(Box::new(
				self.resolve_field(field.type_info(), field.type_path()),
			));
		}

		let variants = info
			.iter()
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
							payload: Some(ValueSchema::Tuple(TupleSchema {
								name: None,
								description: None,
								fields,
							})),
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
							description: None,
							allow_additional: false,
							fields,
						})),
					}
				}
			})
			.collect();
		ValueSchema::Enum(EnumSchema {
			name: Some(SmolStr::from(info.type_path_table().short_path())),
			description,
			variants,
		})
	}
}

fn set_schema(info: &SetInfo) -> ListSchema {
	let item = type_path_schema(info.value_ty().path());
	ListSchema {
		item: Box::new(item),
		min_items: None,
		max_items: None,
		unique: true,
	}
}

/// The type's own doc comment, the type-level twin of a field's description.
#[cfg(feature = "bevy_reflect_documentation")]
fn type_docs(type_info: &TypeInfo) -> Option<SmolStr> {
	type_info.docs().map(SmolStr::from)
}
#[cfg(not(feature = "bevy_reflect_documentation"))]
fn type_docs(_type_info: &TypeInfo) -> Option<SmolStr> { None }

/// The schema for a type known only by its path: an opaque type, or a field
/// whose type carries no [`TypeInfo`].
///
/// The standard containers are recognized by base name, so a `Vec<String>` with
/// no type info still lowers to a list of strings rather than losing its shape.
/// An unrecognized type is [`ValueSchema::Any`], the "nothing is known about
/// this" schema: a `Null` would instead reject every non-null value the type
/// actually holds.
fn type_path_schema(type_path: &str) -> ValueSchema {
	match generic_parts(type_path) {
		Some(("Option", inner)) => {
			return ValueSchema::Optional(Box::new(type_path_schema(inner)));
		}
		Some(("Vec", inner)) => {
			return ValueSchema::List(ListSchema {
				item: Box::new(type_path_schema(inner)),
				min_items: None,
				max_items: None,
				unique: false,
			});
		}
		Some(("HashSet" | "BTreeSet", inner)) => {
			return ValueSchema::List(ListSchema {
				item: Box::new(type_path_schema(inner)),
				min_items: None,
				max_items: None,
				unique: true,
			});
		}
		Some(("HashMap" | "BTreeMap", args)) => {
			return ValueSchema::Map(MapSchema {
				value: Box::new(type_path_schema(map_value_arg(args))),
			});
		}
		_ => {}
	}
	let short = type_path
		.rsplit("::")
		.next()
		.unwrap_or(type_path)
		.trim_start_matches('&');
	match short {
		// `Duration` reflects as opaque but is authored as a unit-suffixed string
		// (eg `"30s"`), coerced by `scalar_to_reflect`, so validate it as a string.
		"String" | "str" | "char" | "Cow<str>" | "PathBuf" | "OsString"
		| "SmolStr" | "SmolPath" | "Duration" => {
			ValueSchema::String(StringSchema::default())
		}
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
		_ => ValueSchema::Any,
	}
}

/// The base name and generic arguments of a path like `alloc::vec::Vec<T>`,
/// ie `("Vec", "T")`.
fn generic_parts(type_path: &str) -> Option<(&str, &str)> {
	let path = type_path.trim();
	let open = path.find('<')?;
	let args = path[open + 1..].strip_suffix('>')?;
	Some((path[..open].rsplit("::").next()?, args))
}

/// The value argument of a map's `K, V` argument list, splitting at the one
/// comma that is not itself inside a generic.
fn map_value_arg(args: &str) -> &str {
	let mut depth = 0;
	for (idx, char) in args.char_indices() {
		match char {
			'<' => depth += 1,
			'>' => depth -= 1,
			',' if depth == 0 => return args[idx + 1..].trim(),
			_ => {}
		}
	}
	args.trim()
}

/// Whether a field must be present: everything but an `Option`, which is
/// [`ValueSchema::Optional`] and validates when absent or null.
fn is_required_field(type_path: &str) -> bool {
	!matches!(generic_parts(type_path), Some(("Option", _)))
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

	/// A field whose type carries no [`TypeInfo`] is known only by its path, and
	/// the standard containers still lower to their shape rather than to `Any`.
	#[crate::test]
	fn a_type_path_alone_still_names_a_shape() {
		use super::type_path_schema;
		type_path_schema("alloc::string::String")
			.xpect_eq(ValueSchema::String(default()));
		type_path_schema("smol_str::SmolStr")
			.xpect_eq(ValueSchema::String(default()));
		type_path_schema("core::time::Duration")
			.xpect_eq(ValueSchema::String(default()));
		type_path_schema("u32").xpect_eq(ValueSchema::U64(default()));
		type_path_schema("i64").xpect_eq(ValueSchema::I64(default()));
		type_path_schema("f64").xpect_eq(ValueSchema::F64(default()));
		type_path_schema("bool").xpect_eq(ValueSchema::Bool(default()));
		type_path_schema("()").xpect_eq(ValueSchema::Null);
		type_path_schema("bevy_ecs::entity::Entity")
			.xpect_eq(ValueSchema::Entity(default()));
		// nothing is known about an unregistered opaque type, which is `Any`
		// rather than a `Null` that would reject every value it holds
		type_path_schema("my_crate::MyCustomType").xpect_eq(ValueSchema::Any);

		type_path_schema("alloc::vec::Vec<alloc::string::String>").xpect_eq(
			ValueSchema::List(ListSchema {
				item: Box::new(ValueSchema::String(default())),
				min_items: None,
				max_items: None,
				unique: false,
			}),
		);
		type_path_schema("std::collections::HashSet<u32>").xpect_eq(
			ValueSchema::List(ListSchema {
				item: Box::new(ValueSchema::U64(default())),
				min_items: None,
				max_items: None,
				unique: true,
			}),
		);
		// the value type is what a map validates, past the key argument
		type_path_schema("bevy::platform::collections::HashMap<String, bool>")
			.xpect_eq(ValueSchema::Map(MapSchema {
				value: Box::new(ValueSchema::Bool(default())),
			}));
		type_path_schema("core::option::Option<bool>").xpect_eq(
			ValueSchema::Optional(Box::new(ValueSchema::Bool(default()))),
		);
	}

	/// Only an `Option` may be absent; the check reads the path because a field
	/// is asked about before its type is walked.
	#[crate::test]
	fn only_an_option_field_is_optional() {
		use super::is_required_field;
		is_required_field("alloc::string::String").xpect_true();
		is_required_field("i32").xpect_true();
		is_required_field("core::option::Option<String>").xpect_false();
		is_required_field("Option<i32>").xpect_false();
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
