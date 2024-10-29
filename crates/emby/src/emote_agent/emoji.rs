use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Serialize, Deserialize)]
pub struct Emoji {
	emoji: String,
	hexcode: String,
	group: String,
	subgroups: String,
	annotation: String,
	tags: String,
	openmoji_tags: String,
	openmoji_author: String,
	openmoji_date: String,
	skintone: Option<u32>,
	skintone_combination: String,
	skintone_base_emoji: String,
	skintone_base_hexcode: String,
	unicode: Option<f32>,
	order: Option<u32>,
}

impl Into<Emoji> for EmojiRaw {
	fn into(self) -> Emoji {
		Emoji {
			emoji: self.emoji,
			hexcode: self.hexcode,
			group: self.group,
			subgroups: self.subgroups,
			annotation: self.annotation,
			tags: self.tags,
			openmoji_tags: self.openmoji_tags,
			openmoji_author: self.openmoji_author,
			openmoji_date: self.openmoji_date,
			skintone: match self.skintone {
				StringOrU32::U32(u) => Some(u),
				_ => None,
			},
			skintone_combination: self.skintone_combination,
			skintone_base_emoji: self.skintone_base_emoji,
			skintone_base_hexcode: self.skintone_base_hexcode,
			unicode: match self.unicode {
				StringOrF32::F32(f) => Some(f),
				_ => None,
			},
			order: match self.order {
				StringOrU32::U32(u) => Some(u),
				_ => None,
			},
		}
	}
}

/// Emojis as they are in the raw json file, null values are strings
#[derive(Debug, Serialize, Deserialize)]
pub struct EmojiRaw {
	emoji: String,
	hexcode: String,
	group: String,
	subgroups: String,
	annotation: String,
	tags: String,
	openmoji_tags: String,
	openmoji_author: String,
	openmoji_date: String,
	skintone: StringOrU32,
	skintone_combination: String,
	skintone_base_emoji: String,
	skintone_base_hexcode: String,
	unicode: StringOrF32,
	order: StringOrU32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum StringOrU32 {
	String(String),
	U32(u32),
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum StringOrF32 {
	String(String),
	F32(f32),
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;

	#[test]
	fn works() -> Result<()> {
		let str =
			std::fs::read_to_string("../../assets/openmoji/openmoji.json")?;

		let _emojis = serde_json::from_str::<Vec<EmojiRaw>>(&str)?;
		let _emojis2: Vec<Emoji> =
			_emojis.into_iter().map(|e| e.into()).collect();

		Ok(())
	}
}
