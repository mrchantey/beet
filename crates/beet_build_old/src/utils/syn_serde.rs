use quote::ToTokens;
use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;
use serde::Serializer;
use syn::Item;


/// Serialization and deserialization helpers for syn::Item
pub mod syn_item_serde {
	use super::*;

	pub fn serialize<S>(item: &Item, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let item_as_string = item.to_token_stream().to_string();
		item_as_string.serialize(serializer)
	}

	pub fn deserialize<'de, D>(deserializer: D) -> Result<Item, D::Error>
	where
		D: Deserializer<'de>,
	{
		let s = String::deserialize(deserializer)?;
		syn::parse_str::<Item>(&s).map_err(|e| {
			serde::de::Error::custom(format!("Failed to parse item: {}", e))
		})
	}
}

/// Serialization and deserialization helpers for Vec<syn::Item>
pub mod syn_item_vec_serde {
	use super::*;

	pub fn serialize<S>(
		items: &[Item],
		serializer: S,
	) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		// Convert each Item to a String, then serialize the Vec<String>
		let string_items: Vec<String> = items
			.iter()
			.map(|item| item.to_token_stream().to_string())
			.collect();
		string_items.serialize(serializer)
	}

	pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<Item>, D::Error>
	where
		D: Deserializer<'de>,
	{
		// Deserialize to Vec<String> and then parse each string
		let string_items = Vec::<String>::deserialize(deserializer)?;
		string_items
			.into_iter()
			.map(|s| {
				syn::parse_str::<Item>(&s).map_err(|e| {
					serde::de::Error::custom(format!(
						"Failed to parse item: {}",
						e
					))
				})
			})
			.collect()
	}
}



#[cfg(test)]
mod test {
	use super::syn_item_serde;
	use quote::ToTokens;
	use serde::Deserialize;
	use serde::Serialize;
	use sweet::prelude::*;
	use syn::Item;

	// Import the module we're testing

	#[derive(Debug, Serialize, Deserialize)]
	struct ItemWrapper {
		#[serde(with = "syn_item_serde")]
		item: Item,
	}

	#[sweet::test]
	fn test_item_serde_roundtrip() {
		// Create a simple Item to test with
		let code = "fn test_function() { println!(\"Hello, world!\"); }";
		let original_item = syn::parse_str::<Item>(code)
			.expect("Failed to parse test function");

		// Create a wrapper with our item
		let wrapper = ItemWrapper {
			item: original_item,
		};

		// Serialize to JSON
		let serialized = serde_json::to_string(&wrapper).unwrap();

		// Deserialize back
		let deserialized: ItemWrapper =
			serde_json::from_str(&serialized).unwrap();

		// Convert both to strings for comparison
		let original_string = wrapper.item.to_token_stream().to_string();
		let deserialized_string =
			deserialized.item.to_token_stream().to_string();

		// They should match
		expect(deserialized_string).to_be(original_string);

		// Deserialization should fail with invalid syntax
		let invalid_json = r#"{"item": "fn invalid syntax {"}"#;
		serde_json::from_str::<ItemWrapper>(invalid_json)
			.xpect()
			.to_be_err();
	}

	#[sweet::test]
	fn test_serialize_complex_item() {
		let complex_item: Item = syn::parse_quote! {
			#[derive(Debug)]
			struct TestStruct {
					field1: i32,
					field2: String,
			}
		};

		let wrapper = ItemWrapper { item: complex_item };

		// Verify we can serialize and deserialize without errors
		serde_json::to_string(&wrapper)
			.unwrap()
			.xmap(|json| serde_json::from_str::<ItemWrapper>(&json))
			.xpect()
			.to_be_ok();
	}
}
