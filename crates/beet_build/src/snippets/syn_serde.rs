use beet_core::prelude::Unspan;
use quote::ToTokens;
use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;
use serde::Serializer;
use syn::Expr;
use syn::Item;
use syn::Type;

/// Macro to generate serialization and deserialization helpers for syn types
macro_rules! syn_serde_mod {
	($mod_name:ident, $ty:ty) => {
		/// Serialization and deserialization helpers,
		/// for option see the [`option`] submodule.
		pub mod $mod_name {
			use super::*;
			pub fn serialize<S>(
				val: &Unspan<$ty>,
				serializer: S,
			) -> Result<S::Ok, S::Error>
			where
				S: Serializer,
			{
				let val_str = val.to_token_stream().to_string();
				val_str.serialize(serializer)
			}
			pub fn deserialize<'de, D>(
				deserializer: D,
			) -> Result<Unspan<$ty>, D::Error>
			where
				D: Deserializer<'de>,
			{
				let val_str = String::deserialize(deserializer)?;
				Unspan::<$ty>::parse_str(&val_str)
					.map_err(|e| serde::de::Error::custom(e.to_string()))
			}
			pub mod option {
				use super::*;
				pub fn serialize<S>(
					val: &Option<Unspan<$ty>>,
					serializer: S,
				) -> Result<S::Ok, S::Error>
				where
					S: Serializer,
				{
					match val {
						Some(inner) => {
							let val_str = inner.to_token_stream().to_string();
							serializer.serialize_some(&val_str)
						}
						None => serializer.serialize_none(),
					}
				}
				pub fn deserialize<'de, D>(
					deserializer: D,
				) -> Result<Option<Unspan<$ty>>, D::Error>
				where
					D: Deserializer<'de>,
				{
					let opt = Option::<String>::deserialize(deserializer)?;
					opt.map(|val_str| {
						Unspan::<$ty>::parse_str(&val_str).map_err(|e| {
							serde::de::Error::custom(e.to_string())
						})
					})
					.transpose()
				}
			}
		}
	};
}

syn_serde_mod!(syn_type_serde, Type);
syn_serde_mod!(syn_item_serde, Item);
syn_serde_mod!(syn_expr_serde, Expr);

/// Serialization and deserialization helpers for Vec<Unspan<syn::Item>>
pub mod syn_item_vec_serde {
	use super::*;

	pub fn serialize<S>(
		items: &[Unspan<Item>],
		serializer: S,
	) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let string_items: Vec<String> = items
			.iter()
			.map(|item| item.to_token_stream().to_string())
			.collect();
		string_items.serialize(serializer)
	}

	pub fn deserialize<'de, D>(
		deserializer: D,
	) -> Result<Vec<Unspan<Item>>, D::Error>
	where
		D: Deserializer<'de>,
	{
		let string_items = Vec::<String>::deserialize(deserializer)?;
		string_items
			.into_iter()
			.map(|s| {
				Unspan::<Item>::parse_str(&s).map_err(|e| {
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
	use beet_core::prelude::*;
	use quote::ToTokens;
	use serde::Deserialize;
	use serde::Serialize;
	use sweet::prelude::*;
	use syn::Item;

	#[derive(Debug, Serialize, Deserialize)]
	struct ItemWrapper {
		#[serde(with = "syn_item_serde")]
		item: Unspan<Item>,
	}

	#[sweet::test]
	fn test_item_serde_roundtrip() {
		let original_item: Item = syn::parse_quote!(
			fn test_function() {
				println!("Hello, world!");
			}
		);

		let wrapper = ItemWrapper {
			item: Unspan::new(&original_item),
		};

		let serialized = serde_json::to_string(&wrapper).unwrap();

		let deserialized: ItemWrapper =
			serde_json::from_str(&serialized).unwrap();

		let original_string = wrapper.item.to_token_stream().to_string();
		let deserialized_string =
			deserialized.item.to_token_stream().to_string();

		deserialized_string.xpect_eq(original_string);

		let invalid_json = r#"{"item": "fn invalid syntax {"}"#;
		serde_json::from_str::<ItemWrapper>(invalid_json).xpect_err();
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

		let wrapper = ItemWrapper {
			item: Unspan::new(&complex_item),
		};

		serde_json::to_string(&wrapper)
			.unwrap()
			.xmap(|json| serde_json::from_str::<ItemWrapper>(&json))
			.xpect_ok();
	}
}
