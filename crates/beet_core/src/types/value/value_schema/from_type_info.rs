//! Conversion from bevy reflect [`TypeInfo`] to [`ValueSchema`].
//!
//! Mirrors [`crate::types::value::schema`] (JSON Schema) but produces a
//! [`ValueSchema`] suitable for validation and UI generation.
use super::*;
use crate::prelude::*;
use bevy::reflect::NamedField;
use bevy::reflect::TypeInfo;
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

/// Builds a [`ValueSchema`] from a bevy reflect [`TypeInfo`].
pub fn build(type_info: &TypeInfo) -> ValueSchema {
	match type_info {
		TypeInfo::Struct(info) => ValueSchema::Struct(struct_schema(info)),
		TypeInfo::TupleStruct(info) => tuple_struct_schema(info),
		TypeInfo::Tuple(info) => {
			if info.field_len() == 0 {
				ValueSchema::Null
			} else {
				ValueSchema::Tuple(tuple_schema(info, None))
			}
		}
		TypeInfo::List(info) => ValueSchema::List(list_schema(info)),
		TypeInfo::Array(info) => ValueSchema::List(array_schema(info)),
		TypeInfo::Map(info) => ValueSchema::Map(map_schema(info)),
		TypeInfo::Set(info) => ValueSchema::List(set_schema(info)),
		TypeInfo::Enum(info) => enum_schema(info),
		TypeInfo::Opaque(info) => primitive_schema(info.type_path()),
	}
}

fn resolve_field(type_info: Option<&TypeInfo>, type_path: &str) -> ValueSchema {
	match type_info {
		Some(info) => build(info),
		None => primitive_schema(type_path),
	}
}

fn struct_schema(info: &StructInfo) -> StructSchema {
	let fields = info.iter().map(named_field_schema).collect();
	StructSchema {
		name: Some(SmolStr::from(info.type_path_table().short_path())),
		allow_additional: false,
		fields,
	}
}

fn named_field_schema(field: &NamedField) -> NamedFieldSchema {
	let required = is_required_field(field.type_path());
	let schema = resolve_field(field.type_info(), field.type_path());

	#[cfg(feature = "bevy_reflect_documentation")]
	let description = field.docs().map(SmolStr::from);
	#[cfg(not(feature = "bevy_reflect_documentation"))]
	let description = None;

	NamedFieldSchema {
		key: SmolStr::from(field.name()),
		required,
		label: None,
		description,
		schema,
	}
}

fn unnamed_field_schema(field: &UnnamedField) -> UnnamedFieldSchema {
	let required = is_required_field(field.type_path());
	let schema = resolve_field(field.type_info(), field.type_path());

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

fn tuple_struct_schema(info: &TupleStructInfo) -> ValueSchema {
	// Newtypes unwrap to their inner type.
	if info.field_len() == 1 {
		let field = info.field_at(0).expect("tuple struct has 1 field");
		return resolve_field(field.type_info(), field.type_path());
	}
	let fields = info.iter().map(unnamed_field_schema).collect();
	ValueSchema::Tuple(TupleSchema {
		name: Some(SmolStr::from(info.type_path_table().short_path())),
		fields,
	})
}

fn tuple_schema(info: &TupleInfo, name: Option<SmolStr>) -> TupleSchema {
	let fields = info.iter().map(unnamed_field_schema).collect();
	TupleSchema { name, fields }
}

fn list_schema(info: &ListInfo) -> ListSchema {
	let item = resolve_field(info.item_info(), info.item_ty().path());
	ListSchema {
		item: Box::new(item),
		min_items: None,
		max_items: None,
		unique: false,
	}
}

fn array_schema(info: &ArrayInfo) -> ListSchema {
	let item = resolve_field(info.item_info(), info.item_ty().path());
	ListSchema {
		item: Box::new(item),
		min_items: Some(info.capacity()),
		max_items: Some(info.capacity()),
		unique: false,
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

fn map_schema(info: &MapInfo) -> MapSchema {
	let value = resolve_field(info.value_info(), info.value_ty().path());
	MapSchema {
		value: Box::new(value),
	}
}

fn enum_schema(info: &EnumInfo) -> ValueSchema {
	// Treat `Option<T>` specially: schema becomes the inner type, since
	// values are flattened by serde-style external tagging.
	if is_option_type(info.type_path())
		&& let Some(VariantInfo::Tuple(some_info)) = info.variant("Some")
		&& let Some(field) = some_info.field_at(0)
	{
		return resolve_field(field.type_info(), field.type_path());
	}

	let variants = info
		.iter()
		.map(|variant| match variant {
			VariantInfo::Unit(v) => VariantSchema {
				name: SmolStr::from(v.name()),
				payload: None,
			},
			VariantInfo::Tuple(v) => {
				if v.field_len() == 1 {
					let field = v.field_at(0).expect("len == 1");
					VariantSchema {
						name: SmolStr::from(v.name()),
						payload: Some(resolve_field(
							field.type_info(),
							field.type_path(),
						)),
					}
				} else {
					let fields = v.iter().map(unnamed_field_schema).collect();
					VariantSchema {
						name: SmolStr::from(v.name()),
						payload: Some(ValueSchema::Tuple(TupleSchema {
							name: None,
							fields,
						})),
					}
				}
			}
			VariantInfo::Struct(v) => {
				let fields = v.iter().map(named_field_schema).collect();
				VariantSchema {
					name: SmolStr::from(v.name()),
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

fn primitive_schema(type_path: &str) -> ValueSchema {
	if is_option_type(type_path) {
		// Outer Option<T> path with no resolvable inner type info: treat as
		// the inner primitive (best effort).
		if let Some(inner) = extract_option_inner(type_path) {
			return primitive_schema(inner);
		}
	}
	let short = type_path.rsplit("::").next().unwrap_or(type_path);
	match short {
		"String" | "str" | "char" | "PathBuf" | "OsString" => {
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
