use crate::prelude::*;
use bevy::reflect::DynamicList;
use bevy::reflect::DynamicStruct;
use bevy::reflect::DynamicTupleStruct;
use bevy::reflect::FromReflect;
use bevy::reflect::PartialReflect;
use bevy::reflect::ReflectRef;
use bevy::reflect::StructInfo;
use bevy::reflect::TupleStructInfo;
use bevy::reflect::TypeInfo;
use bevy::reflect::Typed;
use core::any::TypeId;

#[allow(unused)]
pub fn type_to_value<T>(value: &T) -> Result<Value>
where
	T: 'static + Send + Sync + Reflect,
{
	reflect_to_value(value)
}

/// Convert a reflected value to a [`Value`].
pub fn reflect_to_value(reflect: &dyn PartialReflect) -> Result<Value> {
	// Handle primitives first by trying to downcast
	if let Some(val) = reflect.try_downcast_ref::<bool>() {
		return Ok(Value::Bool(*val));
	}
	if let Some(val) = reflect.try_downcast_ref::<String>() {
		return Ok(Value::str(val));
	}
	if let Some(val) = reflect.try_downcast_ref::<&str>() {
		return Ok(Value::str(*val));
	}

	// Unsigned integers
	if let Some(val) = reflect.try_downcast_ref::<u8>() {
		return Ok(Value::Uint(*val as u64));
	}
	if let Some(val) = reflect.try_downcast_ref::<u16>() {
		return Ok(Value::Uint(*val as u64));
	}
	if let Some(val) = reflect.try_downcast_ref::<u32>() {
		return Ok(Value::Uint(*val as u64));
	}
	if let Some(val) = reflect.try_downcast_ref::<u64>() {
		return Ok(Value::Uint(*val));
	}
	if let Some(val) = reflect.try_downcast_ref::<u128>() {
		return Ok(Value::Uint(*val as u64));
	}
	if let Some(val) = reflect.try_downcast_ref::<usize>() {
		return Ok(Value::Uint(*val as u64));
	}

	// Signed integers
	if let Some(val) = reflect.try_downcast_ref::<i8>() {
		return Ok(Value::Int(*val as i64));
	}
	if let Some(val) = reflect.try_downcast_ref::<i16>() {
		return Ok(Value::Int(*val as i64));
	}
	if let Some(val) = reflect.try_downcast_ref::<i32>() {
		return Ok(Value::Int(*val as i64));
	}
	if let Some(val) = reflect.try_downcast_ref::<i64>() {
		return Ok(Value::Int(*val));
	}
	if let Some(val) = reflect.try_downcast_ref::<i128>() {
		return Ok(Value::Int(*val as i64));
	}
	if let Some(val) = reflect.try_downcast_ref::<isize>() {
		return Ok(Value::Int(*val as i64));
	}

	// Floats
	if let Some(val) = reflect.try_downcast_ref::<f32>() {
		return Ok(Value::Float(*val as f64));
	}
	if let Some(val) = reflect.try_downcast_ref::<f64>() {
		return Ok(Value::Float(*val));
	}

	// Bytes — check specifically for Vec<u8> before generic list handling
	if let Some(val) = reflect.try_downcast_ref::<Vec<u8>>() {
		return Ok(Value::Bytes(val.clone()));
	}

	// Handle complex types via reflection
	match reflect.reflect_ref() {
		ReflectRef::Struct(s) => {
			let mut map = Map::default();
			for idx in 0..s.field_len() {
				let field_name = s.name_at(idx).ok_or_else(|| {
					bevyhow!("struct field at index {} has no name", idx)
				})?;
				let field_value = s.field_at(idx).ok_or_else(|| {
					bevyhow!("struct field at index {} not found", idx)
				})?;
				map.insert(field_name.into(), reflect_to_value(field_value)?);
			}
			Ok(Value::Map(map))
		}
		ReflectRef::TupleStruct(ts) => {
			// Single-field tuple structs (newtypes) unwrap to their inner value
			if ts.field_len() == 1 {
				let field = ts.field(0).ok_or_else(|| {
					bevyhow!("tuple struct field 0 not found")
				})?;
				reflect_to_value(field)
			} else {
				let mut list = Vec::with_capacity(ts.field_len());
				for idx in 0..ts.field_len() {
					let field = ts.field(idx).ok_or_else(|| {
						bevyhow!(
							"tuple struct field at index {} not found",
							idx
						)
					})?;
					list.push(reflect_to_value(field)?);
				}
				Ok(Value::List(list))
			}
		}
		ReflectRef::Tuple(t) => {
			let mut list = Vec::with_capacity(t.field_len());
			for idx in 0..t.field_len() {
				let field = t.field(idx).ok_or_else(|| {
					bevyhow!("tuple field at index {} not found", idx)
				})?;
				list.push(reflect_to_value(field)?);
			}
			Ok(Value::List(list))
		}
		ReflectRef::List(l) => {
			let mut list = Vec::with_capacity(l.len());
			for idx in 0..l.len() {
				let item = l.get(idx).ok_or_else(|| {
					bevyhow!("list item at index {} not found", idx)
				})?;
				list.push(reflect_to_value(item)?);
			}
			Ok(Value::List(list))
		}
		ReflectRef::Array(a) => {
			let mut list = Vec::with_capacity(a.len());
			for idx in 0..a.len() {
				let item = a.get(idx).ok_or_else(|| {
					bevyhow!("array item at index {} not found", idx)
				})?;
				list.push(reflect_to_value(item)?);
			}
			Ok(Value::List(list))
		}
		ReflectRef::Map(m) => {
			let mut map = Map::default();
			for (key, value) in m.iter() {
				let key_str = key
					.try_downcast_ref::<String>()
					.map(|s| SmolStr::from(s.as_str()))
					.or_else(|| {
						key.try_downcast_ref::<&str>()
							.map(|s| SmolStr::from(*s))
					})
					.ok_or_else(|| bevyhow!("map key must be a string"))?;
				map.insert(key_str, reflect_to_value(value)?);
			}
			Ok(Value::Map(map))
		}
		ReflectRef::Set(s) => {
			let mut list = Vec::with_capacity(s.len());
			for item in s.iter() {
				list.push(reflect_to_value(item)?);
			}
			Ok(Value::List(list))
		}
		ReflectRef::Enum(e) => {
			// Handle Option<T> specially
			let type_path = reflect
				.get_represented_type_info()
				.map(|info| info.type_path())
				.unwrap_or("");

			if type_path.starts_with("core::option::Option") {
				match e.variant_name() {
					"None" => return Ok(Value::Null),
					"Some" => {
						let field = e.field_at(0).ok_or_else(|| {
							bevyhow!("Option::Some has no field")
						})?;
						return reflect_to_value(field);
					}
					_ => {}
				}
			}

			// Generic enum: create a map with variant name and fields
			let variant_name = e.variant_name();

			match e.variant_type() {
				bevy::reflect::VariantType::Unit => {
					Ok(Value::str(variant_name))
				}
				bevy::reflect::VariantType::Tuple => {
					let mut fields = Vec::with_capacity(e.field_len());
					for idx in 0..e.field_len() {
						let field = e.field_at(idx).ok_or_else(|| {
							bevyhow!(
								"enum tuple field at index {} not found",
								idx
							)
						})?;
						fields.push(reflect_to_value(field)?);
					}
					let mut variant_map = Map::default();
					variant_map.insert(variant_name.into(), fields.into());
					Ok(Value::Map(variant_map))
				}
				bevy::reflect::VariantType::Struct => {
					let mut fields = Map::default();
					for idx in 0..e.field_len() {
						let field_name = e.name_at(idx).ok_or_else(|| {
							bevyhow!(
								"enum struct field at index {} has no name",
								idx
							)
						})?;
						let field = e.field_at(idx).ok_or_else(|| {
							bevyhow!(
								"enum struct field at index {} not found",
								idx
							)
						})?;
						fields.insert(
							field_name.into(),
							reflect_to_value(field)?,
						);
					}
					let mut variant_map = Map::default();
					variant_map.insert(variant_name.into(), fields.into());
					Ok(Value::Map(variant_map))
				}
			}
		}
		ReflectRef::Opaque(_) => {
			bevybail!(
				"cannot convert opaque type to Value: {:?}",
				reflect.reflect_kind()
			)
		}
		// functions with function feature
		#[allow(unreachable_patterns)]
		other => {
			bevybail!("unsupported reflect kind: {:?}", other.kind())
		}
	}
}

pub fn value_to_type<T>(value: &Value) -> Result<T>
where
	T: 'static + Send + Sync + FromReflect + Typed,
{
	let type_info = T::type_info();
	let dynamic = value_to_dynamic(value, type_info)?;
	T::from_reflect(dynamic.as_partial_reflect()).ok_or_else(|| {
		bevyhow!("failed to convert Value to {}", type_info.type_path())
	})
}

/// Build a dynamic reflected value from a [`Value`] and type info.
pub fn value_to_dynamic(
	value: &Value,
	type_info: &TypeInfo,
) -> Result<Box<dyn PartialReflect>> {
	match type_info {
		TypeInfo::Struct(info) => build_dynamic_struct(value, info),
		TypeInfo::TupleStruct(info) => build_dynamic_tuple_struct(value, info),
		TypeInfo::Tuple(info) => build_dynamic_tuple(value, info),
		TypeInfo::List(info) => build_dynamic_list(value, info),
		TypeInfo::Array(info) => build_dynamic_array(value, info),
		TypeInfo::Map(info) => build_dynamic_map(value, info),
		TypeInfo::Set(_) => {
			bevybail!("Set types are not supported for Value → Type conversion")
		}
		TypeInfo::Enum(info) => build_dynamic_enum(value, info),
		TypeInfo::Opaque(info) => build_opaque_value(value, info.type_id()),
	}
}

/// Build a [`DynamicStruct`] from a [`Value::Map`].
fn build_dynamic_struct(
	value: &Value,
	info: &StructInfo,
) -> Result<Box<dyn PartialReflect>> {
	let map = value.as_map()?;

	let mut dynamic = DynamicStruct::default();

	for field_idx in 0..info.field_len() {
		let field = info.field_at(field_idx).ok_or_else(|| {
			bevyhow!("struct field at index {} not found", field_idx)
		})?;
		let field_name = field.name();
		let field_type_id = field.type_id();
		let field_type_info = field.type_info();

		let field_value = map.get(field_name);

		if let Some(built) = build_field_value(
			field_value,
			field_name,
			field_type_id,
			field_type_info,
		)? {
			dynamic.insert_boxed(field_name, built);
		}
	}

	Ok(Box::new(dynamic))
}

/// Build a [`DynamicTupleStruct`] from a [`Value`].
fn build_dynamic_tuple_struct(
	value: &Value,
	info: &TupleStructInfo,
) -> Result<Box<dyn PartialReflect>> {
	let mut dynamic = DynamicTupleStruct::default();

	// Single-field tuple structs (newtypes) unwrap from their inner value
	if info.field_len() == 1 {
		let field = info.field_at(0).ok_or_else(|| {
			bevyhow!("tuple struct field at index 0 not found")
		})?;

		if let Some(built) = build_field_value(
			Some(value),
			"0",
			field.type_id(),
			field.type_info(),
		)? {
			dynamic.insert_boxed(built);
		}
	} else {
		let list = value.as_list()?;

		for field_idx in 0..info.field_len() {
			let field = info.field_at(field_idx).ok_or_else(|| {
				bevyhow!("tuple struct field at index {} not found", field_idx)
			})?;

			let field_value = list.get(field_idx);

			if let Some(built) = build_field_value(
				field_value,
				&field_idx.to_string(),
				field.type_id(),
				field.type_info(),
			)? {
				dynamic.insert_boxed(built);
			}
		}
	}

	Ok(Box::new(dynamic))
}

/// Build a dynamic tuple from a [`Value::List`].
fn build_dynamic_tuple(
	value: &Value,
	info: &bevy::reflect::TupleInfo,
) -> Result<Box<dyn PartialReflect>> {
	let list = value.as_list()?;

	let mut dynamic = bevy::reflect::DynamicTuple::default();

	for field_idx in 0..info.field_len() {
		let field = info.field_at(field_idx).ok_or_else(|| {
			bevyhow!("tuple field at index {} not found", field_idx)
		})?;

		let field_value = list.get(field_idx);

		if let Some(built) = build_field_value(
			field_value,
			&field_idx.to_string(),
			field.type_id(),
			field.type_info(),
		)? {
			dynamic.insert_boxed(built);
		} else {
			bevybail!("tuple field {} is missing", field_idx);
		}
	}

	Ok(Box::new(dynamic))
}

/// Build a dynamic list from a [`Value::List`].
fn build_dynamic_list(
	value: &Value,
	info: &bevy::reflect::ListInfo,
) -> Result<Box<dyn PartialReflect>> {
	let list = value.as_list()?;

	let mut dynamic = DynamicList::default();
	let item_type_info = info.item_info();

	for item in list {
		if let Some(built) = build_field_value(
			Some(item),
			"item",
			info.item_ty().id(),
			item_type_info,
		)? {
			dynamic.push_box(built);
		}
	}

	Ok(Box::new(dynamic))
}

/// Build a dynamic array from a [`Value::List`].
fn build_dynamic_array(
	value: &Value,
	info: &bevy::reflect::ArrayInfo,
) -> Result<Box<dyn PartialReflect>> {
	let list = value.as_list()?;

	let mut dynamic = DynamicList::default();
	let item_type_info = info.item_info();

	for item in list {
		if let Some(built) = build_field_value(
			Some(item),
			"item",
			info.item_ty().id(),
			item_type_info,
		)? {
			dynamic.push_box(built);
		}
	}

	Ok(Box::new(dynamic))
}

/// Build a dynamic map from a [`Value::Map`].
fn build_dynamic_map(
	value: &Value,
	info: &bevy::reflect::MapInfo,
) -> Result<Box<dyn PartialReflect>> {
	use bevy::reflect::Map;

	let map = value.as_map()?;

	let mut dynamic = bevy::reflect::DynamicMap::default();
	let value_type_info = info.value_info();
	let key_type_id = info.key_ty().id();

	for (key, val) in map {
		if let Some(built) = build_field_value(
			Some(val),
			key.as_str(),
			info.value_ty().id(),
			value_type_info,
		)? {
			// Insert key as String if the map's key type is String,
			// otherwise insert as SmolStr.
			let key_box: Box<dyn PartialReflect> =
				if key_type_id == TypeId::of::<String>() {
					Box::new(key.as_str().to_string())
				} else {
					Box::new(key.clone())
				};
			dynamic.insert_boxed(key_box, built);
		}
	}

	Ok(Box::new(dynamic))
}

/// Build a dynamic enum from a [`Value`].
fn build_dynamic_enum(
	value: &Value,
	info: &bevy::reflect::EnumInfo,
) -> Result<Box<dyn PartialReflect>> {
	// Handle Option<T> specially
	let type_path = info.type_path();
	if type_path.starts_with("core::option::Option") {
		if value.is_null() {
			let mut dynamic = bevy::reflect::DynamicEnum::default();
			dynamic.set_variant("None", bevy::reflect::DynamicVariant::Unit);
			return Ok(Box::new(dynamic));
		} else {
			let some_variant = info
				.variant("Some")
				.ok_or_else(|| bevyhow!("Option type missing Some variant"))?;

			let field_info = match some_variant {
				bevy::reflect::VariantInfo::Tuple(tuple_info) => tuple_info
					.field_at(0)
					.ok_or_else(|| bevyhow!("Option::Some missing field 0"))?,
				_ => bevybail!("Option::Some is not a tuple variant"),
			};

			let inner_value = build_field_value(
				Some(value),
				"0",
				field_info.type_id(),
				field_info.type_info(),
			)?
			.ok_or_else(|| bevyhow!("failed to build Option inner value"))?;

			let mut tuple = bevy::reflect::DynamicTuple::default();
			tuple.insert_boxed(inner_value);

			let mut dynamic = bevy::reflect::DynamicEnum::default();
			dynamic.set_variant(
				"Some",
				bevy::reflect::DynamicVariant::Tuple(tuple),
			);
			return Ok(Box::new(dynamic));
		}
	}

	// Generic enum handling
	match value {
		Value::Str(variant_name) => {
			// Unit variant
			let mut dynamic = bevy::reflect::DynamicEnum::default();
			dynamic.set_variant(
				variant_name.as_str(),
				bevy::reflect::DynamicVariant::Unit,
			);
			Ok(Box::new(dynamic))
		}
		Value::Map(map) => {
			// Should have exactly one entry: variant_name → fields
			if map.len() != 1 {
				bevybail!(
					"expected single-entry map for enum variant, found {} entries",
					map.len()
				);
			}

			let (variant_name, fields) = map.iter().next().unwrap();

			let variant_info =
				info.variant(variant_name.as_str()).ok_or_else(|| {
					bevyhow!("unknown enum variant: {}", variant_name)
				})?;

			let mut dynamic = bevy::reflect::DynamicEnum::default();

			match variant_info {
				bevy::reflect::VariantInfo::Unit(_) => {
					dynamic.set_variant(
						variant_name.as_str(),
						bevy::reflect::DynamicVariant::Unit,
					);
				}
				bevy::reflect::VariantInfo::Tuple(tuple_info) => {
					let list = fields.as_list()?;

					let mut tuple = bevy::reflect::DynamicTuple::default();
					for (idx, field_info) in tuple_info.iter().enumerate() {
						let field_value = list.get(idx);
						if let Some(built) = build_field_value(
							field_value,
							&idx.to_string(),
							field_info.type_id(),
							field_info.type_info(),
						)? {
							tuple.insert_boxed(built);
						}
					}
					dynamic.set_variant(
						variant_name.as_str(),
						bevy::reflect::DynamicVariant::Tuple(tuple),
					);
				}
				bevy::reflect::VariantInfo::Struct(struct_info) => {
					let field_map = fields.as_map()?;

					let mut struct_variant =
						bevy::reflect::DynamicStruct::default();
					for field_info in struct_info.iter() {
						let field_value = field_map.get(field_info.name());
						if let Some(built) = build_field_value(
							field_value,
							field_info.name(),
							field_info.type_id(),
							field_info.type_info(),
						)? {
							struct_variant
								.insert_boxed(field_info.name(), built);
						}
					}
					dynamic.set_variant(
						variant_name.as_str(),
						bevy::reflect::DynamicVariant::Struct(struct_variant),
					);
				}
			}

			Ok(Box::new(dynamic))
		}
		_ => {
			bevybail!("expected Str or Map value for enum, found {:?}", value)
		}
	}
}

/// Build an opaque (primitive) value.
fn build_opaque_value(
	value: &Value,
	type_id: TypeId,
) -> Result<Box<dyn PartialReflect>> {
	if type_id == TypeId::of::<bool>() {
		let val = value
			.as_bool()
			.ok_or_else(|| bevyhow!("expected Bool value for bool field"))?;
		return Ok(Box::new(val));
	}

	if type_id == TypeId::of::<String>() {
		let val = value.as_str().ok_or_else(|| {
			bevyhow!("expected Str value for String field, found {:?}", value)
		})?;
		return Ok(Box::new(val.to_string()));
	}

	// Signed integers
	if type_id == TypeId::of::<i8>() {
		let val = value
			.as_i64()
			.ok_or_else(|| bevyhow!("expected integer value for i8 field"))?;
		return Ok(Box::new(val as i8));
	}
	if type_id == TypeId::of::<i16>() {
		let val = value
			.as_i64()
			.ok_or_else(|| bevyhow!("expected integer value for i16 field"))?;
		return Ok(Box::new(val as i16));
	}
	if type_id == TypeId::of::<i32>() {
		let val = value
			.as_i64()
			.ok_or_else(|| bevyhow!("expected integer value for i32 field"))?;
		return Ok(Box::new(val as i32));
	}
	if type_id == TypeId::of::<i64>() {
		let val = value
			.as_i64()
			.ok_or_else(|| bevyhow!("expected integer value for i64 field"))?;
		return Ok(Box::new(val));
	}
	if type_id == TypeId::of::<i128>() {
		let val = value
			.as_i64()
			.ok_or_else(|| bevyhow!("expected integer value for i128 field"))?;
		return Ok(Box::new(val as i128));
	}
	if type_id == TypeId::of::<isize>() {
		let val = value.as_i64().ok_or_else(|| {
			bevyhow!("expected integer value for isize field")
		})?;
		return Ok(Box::new(val as isize));
	}

	// Unsigned integers
	if type_id == TypeId::of::<u8>() {
		let val = value
			.as_u64()
			.ok_or_else(|| bevyhow!("expected integer value for u8 field"))?;
		return Ok(Box::new(val as u8));
	}
	if type_id == TypeId::of::<u16>() {
		let val = value
			.as_u64()
			.ok_or_else(|| bevyhow!("expected integer value for u16 field"))?;
		return Ok(Box::new(val as u16));
	}
	if type_id == TypeId::of::<u32>() {
		let val = value
			.as_u64()
			.ok_or_else(|| bevyhow!("expected integer value for u32 field"))?;
		return Ok(Box::new(val as u32));
	}
	if type_id == TypeId::of::<u64>() {
		let val = value
			.as_u64()
			.ok_or_else(|| bevyhow!("expected integer value for u64 field"))?;
		return Ok(Box::new(val));
	}
	if type_id == TypeId::of::<u128>() {
		let val = value
			.as_u64()
			.ok_or_else(|| bevyhow!("expected integer value for u128 field"))?;
		return Ok(Box::new(val as u128));
	}
	if type_id == TypeId::of::<usize>() {
		let val = value.as_u64().ok_or_else(|| {
			bevyhow!("expected integer value for usize field")
		})?;
		return Ok(Box::new(val as usize));
	}

	// Floats
	if type_id == TypeId::of::<f32>() {
		let val = value
			.as_f64()
			.ok_or_else(|| bevyhow!("expected float value for f32 field"))?;
		return Ok(Box::new(val as f32));
	}
	if type_id == TypeId::of::<f64>() {
		let val = value
			.as_f64()
			.ok_or_else(|| bevyhow!("expected float value for f64 field"))?;
		return Ok(Box::new(val));
	}

	// Bytes
	if type_id == TypeId::of::<Vec<u8>>() {
		let bytes = value.as_bytes().ok_or_else(|| {
			bevyhow!("expected Bytes value for Vec<u8> field")
		})?;
		return Ok(Box::new(bytes.to_vec()));
	}

	bevybail!("unsupported opaque type")
}

/// Build a field value from an optional [`Value`] reference.
///
/// Returns `None` if the field value is `None` and the field can use its
/// default.
fn build_field_value(
	value: Option<&Value>,
	field_name: &str,
	field_type_id: TypeId,
	field_type_info: Option<&TypeInfo>,
) -> Result<Option<Box<dyn PartialReflect>>> {
	let Some(value) = value else {
		return Ok(None);
	};

	// Handle Null as missing
	if value.is_null() {
		return Ok(None);
	}

	// Try opaque types first
	if let Ok(result) = build_opaque_value(value, field_type_id) {
		return Ok(Some(result));
	}

	// Use type info for complex types
	if let Some(type_info) = field_type_info {
		return value_to_dynamic(value, type_info).map(Some);
	}

	bevybail!(
		"cannot build field '{}': no type info and not a primitive",
		field_name
	)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[derive(Debug, Reflect, Default, PartialEq)]
	#[reflect(Default)]
	struct SimpleStruct {
		name: String,
		count: u32,
		active: bool,
	}

	#[test]
	fn from_reflect_simple_struct() {
		let s = SimpleStruct {
			name: "hello".to_string(),
			count: 42,
			active: true,
		};
		let value = reflect_to_value(&s).unwrap();
		let map = value.as_map().unwrap();
		map.get("name").unwrap().as_str().unwrap().xpect_eq("hello");
		map.get("count").unwrap().as_u64().unwrap().xpect_eq(42u64);
		map.get("active").unwrap().as_bool().unwrap().xpect_true();
	}

	#[test]
	fn into_reflect_simple_struct() {
		let mut map = Map::default();
		map.insert("name".into(), Value::Str("world".into()));
		map.insert("count".into(), Value::Uint(7));
		map.insert("active".into(), Value::Bool(false));
		let value = Value::Map(map);
		let s: SimpleStruct = value_to_type(&value).unwrap();
		s.name.xpect_eq("world");
		s.count.xpect_eq(7u32);
		s.active.xpect_false();
	}

	#[test]
	fn roundtrip_simple_struct() {
		let original = SimpleStruct {
			name: "roundtrip".to_string(),
			count: 99,
			active: true,
		};
		let value = reflect_to_value(&original).unwrap();
		let result: SimpleStruct = value_to_type(&value).unwrap();
		result.name.xpect_eq("roundtrip");
		result.count.xpect_eq(99u32);
		result.active.xpect_true();
	}

	#[derive(Debug, Reflect, Default, PartialEq)]
	#[reflect(Default)]
	struct NestedStruct {
		inner: SimpleStruct,
		label: String,
	}

	#[test]
	fn roundtrip_nested_struct() {
		let original = NestedStruct {
			inner: SimpleStruct {
				name: "inner".to_string(),
				count: 1,
				active: false,
			},
			label: "outer".to_string(),
		};
		let value = reflect_to_value(&original).unwrap();
		let result: NestedStruct = value_to_type(&value).unwrap();
		result.label.xpect_eq("outer");
		result.inner.name.xpect_eq("inner");
		result.inner.count.xpect_eq(1u32);
		result.inner.active.xpect_false();
	}

	#[derive(Debug, Reflect, Default, PartialEq)]
	#[reflect(Default)]
	struct WithVec {
		items: Vec<String>,
	}

	#[test]
	fn roundtrip_with_vec() {
		let original = WithVec {
			items: vec!["a".to_string(), "b".to_string()],
		};
		let value = reflect_to_value(&original).unwrap();
		let result: WithVec = value_to_type(&value).unwrap();
		result.items.len().xpect_eq(2);
		result.items[0].xpect_eq("a");
		result.items[1].xpect_eq("b");
	}

	#[derive(Debug, Reflect, Default, PartialEq)]
	#[reflect(Default)]
	struct WithOption {
		maybe: Option<String>,
	}

	#[test]
	fn roundtrip_option_some() {
		let original = WithOption {
			maybe: Some("yes".to_string()),
		};
		let value = reflect_to_value(&original).unwrap();
		let result: WithOption = value_to_type(&value).unwrap();
		result.maybe.unwrap().xpect_eq("yes");
	}

	#[test]
	fn roundtrip_option_none() {
		let original = WithOption { maybe: None };
		let value = reflect_to_value(&original).unwrap();
		let result: WithOption = value_to_type(&value).unwrap();
		result.maybe.xpect_none();
	}
}
