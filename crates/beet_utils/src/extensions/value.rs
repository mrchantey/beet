use bevy::ecs::error::BevyError;
use bevy::ecs::error::Result;
use extend::ext;
use serde_json::Value;

#[ext]
pub impl Value {
	fn set_field(&mut self, field: &str, value: Value) -> Result<&mut Self> {
		if let Some(obj) = self.as_object_mut() {
			obj.insert(field.to_string(), value);
			Ok(self)
		} else {
			Err(BevyError::from(format!("Expected object, got {:?}", self)))
		}
	}


	/// wraps [`Value::as_str`] with helpful error message
	fn to_str(&self) -> Result<&str> {
		self.as_str()
			.ok_or_else(|| format!("Expected string, got {:?}", self).into())
	}
	/// wraps [`Value::as_f64`] with helpful error message
	fn to_f64(&self) -> Result<f64> {
		self.as_f64()
			.ok_or_else(|| format!("Expected f64, got {:?}", self).into())
	}
	/// wraps [`Value::as_i64`] with helpful error message
	fn to_i64(&self) -> Result<i64> {
		self.as_i64()
			.ok_or_else(|| format!("Expected i64, got {:?}", self).into())
	}
	/// wraps [`Value::as_u64`] with helpful error message
	fn to_u64(&self) -> Result<u64> {
		self.as_u64()
			.ok_or_else(|| format!("Expected u64, got {:?}", self).into())
	}
	/// wraps [`Value::as_bool`] with helpful error message
	fn to_bool(&self) -> Result<bool> {
		self.as_bool()
			.ok_or_else(|| format!("Expected bool, got {:?}", self).into())
	}
	/// wraps [`Value::as_array`] with helpful error message
	fn to_array(&self) -> Result<&Vec<Value>> {
		self.as_array()
			.ok_or_else(|| format!("Expected array, got {:?}", self).into())
	}
	/// wraps [`Value::as_object`] with helpful error message
	fn to_object(&self) -> Result<&serde_json::Map<String, Value>> {
		self.as_object()
			.ok_or_else(|| format!("Expected object, got {:?}", self).into())
	}
	/// checks for null with helpful error message
	fn to_null(&self) -> Result {
		self.is_null()
			.then(|| ())
			.ok_or_else(|| format!("Expected null, got {:?}", self).into())
	}


	/// Get a non-null field, returning a helpful error message if it is missing.
	fn field(&self, field_name: &str) -> Result<&Value> {
		match &self[field_name]{
			Value::Null=>{
			Err(format! {"Expected field '{field_name}'\nParent Object: {:?}",self }.into())
			}
			other=>{
					Ok(other)
			}
		}
	}

	/// Get a field as a string, returning a helpful error message if it is
	/// missing or of a different type.
	fn field_str(&self, field_name: &str) -> Result<&str> {
		let field = &self[field_name];
		field
					.as_str()
					.ok_or_else(|| {
						format! {"Expected field '{field_name}' to be string, got '{}'\nParent Object: {:?}",field, self}.into()
					})
	}

	/// Get a field as an i64, returning a helpful error message if it is
	/// missing or of a different type.
	fn field_i64(&self, field_name: &str) -> Result<i64> {
		let field = &self[field_name];
		field
					.as_i64()
					.ok_or_else(|| {
						format! {"Expected field '{field_name}' to be i64, got '{}'\nParent Object: {:?}",field, self}.into()
					})
	}

	/// Get a field as a u64, returning a helpful error message if it is
	/// missing or of a different type.
	fn field_u64(&self, field_name: &str) -> Result<u64> {
		let field = &self[field_name];
		field
					.as_u64()
					.ok_or_else(|| {
						format! {"Expected field '{field_name}' to be u64, got '{}'\nParent Object: {:?}",field, self}.into()
					})
	}

	/// Get a field as an f64, returning a helpful error message if it is
	/// missing or of a different type.
	fn field_f64(&self, field_name: &str) -> Result<f64> {
		let field = &self[field_name];
		field
					.as_f64()
					.ok_or_else(|| {
						format! {"Expected field '{field_name}' to be f64, got '{}'\nParent Object: {:?}",field, self}.into()
					})
	}

	/// Get a field as a bool, returning a helpful error message if it is
	/// missing or of a different type.
	fn field_bool(&self, field_name: &str) -> Result<bool> {
		let field = &self[field_name];
		field
					.as_bool()
					.ok_or_else(|| {
						format! {"Expected field '{field_name}' to be bool, got '{}'\nParent Object: {:?}",field, self}.into()
					})
	}

	/// Get a field as an array, returning a helpful error message if it is
	/// missing or of a different type.
	fn field_array(&self, field_name: &str) -> Result<&Vec<Value>> {
		let field = &self[field_name];
		field
					.as_array()
					.ok_or_else(|| {
						format! {"Expected field '{field_name}' to be array, got '{}'\nParent Object: {:?}",field, self}.into()
					})
	}

	/// Get a field as an object, returning a helpful error message if it is
	/// missing or of a different type.
	fn field_object(
		&self,
		field_name: &str,
	) -> Result<&serde_json::Map<String, Value>> {
		let field = &self[field_name];
		field
					.as_object()
					.ok_or_else(|| {
						format! {"Expected field '{field_name}' to be object, got '{}'\nParent Object: {:?}",field, self}.into()
					})
	}

	/// Get a field as null, returning a helpful error message if it is
	/// missing or of a different type.
	fn field_null(&self, field_name: &str) -> Result {
		let field = &self[field_name];
		field
					.is_null()
					.then(|| ())
					.ok_or_else(|| {
						format! {"Expected field '{field_name}' to be null, got '{}'\nParent Object: {:?}",field, self}.into()
					})
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use serde_json::json;

	#[test]
	fn test_field_str_success() {
		assert_eq!(
			json!({"name": "Alice"}).field_str("name").unwrap(),
			"Alice"
		);
	}

	#[test]
	fn test_field_str_missing_field() {
		assert!(
			json!({"name": "Alice"})
				.field_str("age")
				.unwrap_err()
				.to_string()
				.contains("Expected field 'age' to be string, got 'null'")
		);
	}

	#[test]
	fn test_field_str_wrong_type() {
		assert!(
			json!({"age": 30})
				.field_str("age")
				.unwrap_err()
				.to_string()
				.contains("Expected field 'age' to be string, got '30'")
		);
	}

	#[test]
	fn test_field_i64_success() {
		assert_eq!(json!({"age": 42}).field_i64("age").unwrap(), 42);
	}

	#[test]
	fn test_field_i64_wrong_type() {
		assert!(
			json!({"age": "not a number"})
				.field_i64("age")
				.unwrap_err()
				.to_string()
				.contains("Expected field 'age' to be i64")
		);
	}

	#[test]
	fn test_field_u64_success() {
		assert_eq!(json!({"count": 123u64}).field_u64("count").unwrap(), 123);
	}

	#[test]
	fn test_field_f64_success() {
		assert_eq!(json!({"score": 3.14}).field_f64("score").unwrap(), 3.14);
	}

	#[test]
	fn test_field_bool_success() {
		assert_eq!(json!({"active": true}).field_bool("active").unwrap(), true);
	}

	#[test]
	fn test_field_array_success() {
		let value = json!({"items": [1, 2, 3]});
		let arr = value.field_array("items").unwrap();
		assert_eq!(arr.len(), 3);
	}

	#[test]
	fn test_field_object_success() {
		let value = json!({"meta": {"foo": 1}});
		let obj = value.field_object("meta").unwrap();
		assert_eq!(obj.get("foo").unwrap(), &json!(1));
	}

	#[test]
	fn test_field_null_success() {
		assert!(json!({"gone": null}).field_null("gone").is_ok());
	}

	#[test]
	fn test_field_null_wrong_type() {
		assert!(
			json!({"gone": 1})
				.field_null("gone")
				.unwrap_err()
				.to_string()
				.contains("Expected field 'gone' to be null")
		);
	}
}
