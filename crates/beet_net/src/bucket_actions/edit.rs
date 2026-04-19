use crate::prelude::*;
use beet_core::prelude::*;

/// A single targeted text replacement.
#[derive(Debug, Clone, Reflect, serde::Serialize, serde::Deserialize)]
pub struct TextEdit {
	/// Exact text to find. Must match a unique region of the original file.
	pub old_text: String,
	/// Replacement text.
	pub new_text: String,
}

/// Parameters for editing text in a blob.
#[derive(Debug, Clone, Reflect, serde::Serialize, serde::Deserialize)]
pub struct EditTextParams {
	/// Path to the file to edit.
	pub path: RelPath,
	/// One or more targeted replacements matched against the original file.
	pub edits: Vec<TextEdit>,
}

/// Edit a file using exact text replacement.
///
/// Each `edits[].old_text` must match a unique, non-overlapping region
/// of the original file. All edits are matched against the original content,
/// not applied incrementally.
#[action(route)]
#[derive(Component, Reflect)]
pub async fn EditText(cx: ActionContext<EditTextParams>) -> Result<()> {
	let bucket = cx
		.caller
		.with_state::<AncestorQuery<&Bucket>, _>(|entity, query| {
			query.get(entity).cloned()
		})
		.await?;

	let content = bucket.get(&cx.input.path).await?;
	let mut text = String::from_utf8(content.to_vec())
		.map_err(|err| bevyhow!("file is not valid UTF-8: {err}"))?;

	// validate all edits against original text before applying
	validate_edits(&text, &cx.input.edits)?;

	// apply each replacement once against the (mutating) text
	for edit in &cx.input.edits {
		text = text.replacen(&edit.old_text, &edit.new_text, 1);
	}

	bucket
		.insert(&cx.input.path, text.into_bytes())
		.await
}

/// Validate that all edits are present, unique, and non-overlapping.
fn validate_edits(text: &str, edits: &[TextEdit]) -> Result<()> {
	let mut ranges: Vec<(usize, usize)> = Vec::with_capacity(edits.len());

	for edit in edits {
		if edit.old_text.is_empty() {
			bevybail!("old_text must not be empty");
		}

		let pos = text
			.find(&edit.old_text)
			.ok_or_else(|| bevyhow!("old_text not found in file: {:?}", &edit.old_text))?;
		let end = pos + edit.old_text.len();

		// check uniqueness: ensure old_text appears exactly once
		if text[pos + 1..].contains(&edit.old_text) {
			bevybail!(
				"old_text appears more than once, add more context to make it unique: {:?}",
				&edit.old_text
			);
		}

		ranges.push((pos, end));
	}

	// sort by start position and check for overlaps
	ranges.sort_by_key(|(start, _)| *start);
	for pair in ranges.windows(2) {
		if pair[0].1 > pair[1].0 {
			bevybail!(
				"overlapping edits at byte ranges [{}, {}) and [{}, {})",
				pair[0].0,
				pair[0].1,
				pair[1].0,
				pair[1].1
			);
		}
	}

	Ok(())
}


#[cfg(test)]
mod test {
	use super::*;

	/// Shared helper: write content, apply edits, return resulting text.
	async fn apply_edits(original: &str, edits: Vec<TextEdit>) -> String {
		let bucket = Bucket::temp();
		let path = RelPath::from("file.txt");
		let original = original.to_owned();
		bucket
			.insert(&path, original.clone())
			.await
			.unwrap();

		validate_edits(&original, &edits).unwrap();
		let mut text = original;
		for edit in &edits {
			text = text.replacen(&edit.old_text, &edit.new_text, 1);
		}
		bucket.insert(&path, text.clone().into_bytes()).await.unwrap();
		let got = bucket.get(&path).await.unwrap();
		String::from_utf8(got.to_vec()).unwrap()
	}

	#[beet_core::test]
	async fn single_edit() {
		let result = apply_edits(
			"hello world",
			vec![TextEdit {
				old_text: "world".into(),
				new_text: "rust".into(),
			}],
		)
		.await;
		result.xpect_eq("hello rust".to_string());
	}

	#[beet_core::test]
	async fn multiple_edits() {
		let result = apply_edits(
			"aaa bbb ccc",
			vec![
				TextEdit {
					old_text: "aaa".into(),
					new_text: "xxx".into(),
				},
				TextEdit {
					old_text: "ccc".into(),
					new_text: "zzz".into(),
				},
			],
		)
		.await;
		result.xpect_eq("xxx bbb zzz".to_string());
	}

	#[test]
	fn rejects_missing_old_text() {
		let edits = vec![TextEdit {
			old_text: "nonexistent".into(),
			new_text: "foo".into(),
		}];
		validate_edits("some text", &edits).xpect_err();
	}

	#[test]
	fn rejects_ambiguous_old_text() {
		let edits = vec![TextEdit {
			old_text: "ab".into(),
			new_text: "xx".into(),
		}];
		validate_edits("ab cd ab", &edits).xpect_err();
	}

	#[test]
	fn rejects_overlapping_edits() {
		let edits = vec![
			TextEdit {
				old_text: "hello wor".into(),
				new_text: "x".into(),
			},
			TextEdit {
				old_text: "lo world".into(),
				new_text: "y".into(),
			},
		];
		validate_edits("hello world", &edits).xpect_err();
	}

	#[test]
	fn rejects_empty_old_text() {
		let edits = vec![TextEdit {
			old_text: "".into(),
			new_text: "foo".into(),
		}];
		validate_edits("some text", &edits).xpect_err();
	}
}
