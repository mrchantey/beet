//! A vault's plaintext in any format, and its envelope.

use crate::prelude::*;

/// What a vault holds once decrypted: a flat [`EnvDocument`], or a tree
/// ([`Value`]) addressed by dotted path and carrying a `meta` table.
///
/// The tree half is one shape rendered as TOML or JSON by the vault's
/// [`VaultFormat`]; the env half is deliberately the simple `.env` grammar
/// and stays flat, because its one job is to become the process environment.
///
/// ## Example
///
/// ```
/// # use beet_core::prelude::*;
/// let mut doc = VaultDocument::new(VaultFormat::Toml);
/// doc.set("secrets.dkim.value", "abc").unwrap();
/// doc.set_meta("app", "mail").unwrap();
/// doc.get("secrets.dkim.value").xpect_eq(Some(Value::str("abc")));
/// doc.keys().xpect_eq(vec![SmolStr::new("secrets.dkim.value")]);
/// let text = doc.render(VaultFormat::Toml).unwrap();
/// VaultDocument::parse(VaultFormat::Toml, &text)
/// 	.unwrap()
/// 	.xpect_eq(doc);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum VaultDocument {
	/// Flat `KEY=value` lines.
	Env(EnvDocument),
	/// A tree of tables, the root a map.
	Tree(Value),
}

impl VaultDocument {
	/// The table a tree document keeps its metadata in.
	pub const META: &'static str = "meta";

	/// An empty document of `format`'s kind.
	pub fn new(format: VaultFormat) -> Self {
		match format {
			VaultFormat::Env => Self::Env(EnvDocument::default()),
			VaultFormat::Toml | VaultFormat::Json => Self::Tree(Value::map()),
		}
	}

	/// Parse `text` as `format`.
	pub fn parse(format: VaultFormat, text: &str) -> Result<Self> {
		match format {
			VaultFormat::Env => Self::Env(EnvDocument::parse(text)),
			VaultFormat::Toml => toml::from_str::<Value>(text)
				.map_err(|err| bevyhow!("invalid toml vault: {err}"))?
				.xmap(Self::Tree),
			VaultFormat::Json => serde_json::from_str::<Value>(text)
				.map_err(|err| bevyhow!("invalid json vault: {err}"))?
				.xmap(Self::Tree),
		}
		.xok()
	}

	/// Render as `format`. A tree renders as any tree format; an env
	/// document renders as a tree of strings; a tree renders as env only when
	/// every value is a flat scalar.
	pub fn render(&self, format: VaultFormat) -> Result<String> {
		match (self, format) {
			(Self::Env(doc), VaultFormat::Env) => doc.to_string().xok(),
			(Self::Env(doc), format) => {
				Self::Tree(Value::Map(Map::new(doc.pairs()))).render(format)
			}
			(Self::Tree(value), VaultFormat::Toml) => {
				toml::to_string_pretty(value)
					.map_err(|err| bevyhow!("cannot render as toml: {err}"))
			}
			(Self::Tree(value), VaultFormat::Json) => {
				value.to_string_pretty().map(|text| text + "\n")
			}
			(Self::Tree(value), VaultFormat::Env) => {
				let mut doc = EnvDocument::default();
				for (key, value) in value.as_map()?.iter() {
					doc.set(key.clone(), Self::scalar_text(key, value)?);
				}
				doc.to_string().xok()
			}
		}
	}

	/// Decrypt `ciphertext` (armored or binary) with `identities` and parse
	/// it as `format`.
	pub fn decrypt(
		format: VaultFormat,
		identities: &AgeIdentityFile,
		ciphertext: &[u8],
	) -> Result<Self> {
		let plaintext = identities.decrypt(ciphertext)?;
		let text = String::from_utf8(plaintext)
			.map_err(|_| bevyhow!("the vault's plaintext is not utf-8"))?;
		Self::parse(format, &text)
	}

	/// Render as `format` and encrypt to `recipients`, as armored text. The
	/// plaintext is bytes in memory, never on disk.
	pub fn encrypt(
		&self,
		format: VaultFormat,
		recipients: &[AgeRecipient],
	) -> Result<String> {
		AgeRecipient::encrypt(recipients, self.render(format)?.as_bytes())
	}

	/// The value at `key`: a flat key on an env document, a dotted path
	/// (`secrets.dkim.value`) on a tree.
	pub fn get(&self, key: &str) -> Option<Value> {
		match self {
			Self::Env(doc) => doc.get(key).map(Value::str),
			Self::Tree(value) => {
				value.get_path(&FieldPath::parse(key)).cloned()
			}
		}
	}

	/// Set `key` to `value`: a tree creates the tables along a dotted path;
	/// an env document takes a scalar only.
	pub fn set(&mut self, key: &str, value: impl Into<Value>) -> Result<()> {
		let value = value.into();
		match self {
			Self::Env(doc) => {
				doc.set(key, Self::scalar_text(key, &value)?);
			}
			Self::Tree(root) => {
				let path = FieldPath::parse(key);
				let Some((leaf, tables)) = path.split_last() else {
					bevybail!("a key is required");
				};
				let mut current = root;
				for segment in tables {
					current = Self::table_mut(current, segment)?;
				}
				current.insert(Self::key_of(leaf)?, value)?;
			}
		}
		Ok(())
	}

	/// Remove `key`, `true` when it existed; a table left empty on a tree is
	/// removed with it, so a vault never renders an empty `[table]`.
	pub fn remove(&mut self, key: &str) -> bool {
		match self {
			Self::Env(doc) => doc.remove(key),
			Self::Tree(root) => {
				let path = FieldPath::parse(key);
				let Some((leaf, tables)) = path.split_last() else {
					return false;
				};
				let FieldSegment::ObjectKey(key) = leaf else {
					return false;
				};
				let removed = root
					.get_path_mut(tables)
					.and_then(|table| table.as_map_mut().ok())
					.and_then(|table| table.remove(key))
					.is_some();
				// prune upwards, the root table excepted
				for depth in (1..=tables.len()).rev() {
					let (parents, last) = tables[..depth].split_at(depth - 1);
					let FieldSegment::ObjectKey(name) = &last[0] else {
						break;
					};
					let Some(parent) = root
						.get_path_mut(parents)
						.and_then(|parent| parent.as_map_mut().ok())
					else {
						break;
					};
					match parent
						.get(name)
						.ok()
						.and_then(|table| table.as_map().ok())
					{
						Some(table) if table.is_empty() => {
							parent.remove(name);
						}
						_ => break,
					}
				}
				removed
			}
		}
	}

	/// Every key: the flat keys of an env document, the dotted path of every
	/// leaf of a tree, the `meta` table excluded.
	pub fn keys(&self) -> Vec<SmolStr> {
		match self {
			Self::Env(doc) => doc.keys(),
			Self::Tree(root) => {
				let mut keys = Vec::new();
				let Ok(map) = root.as_map() else {
					return keys;
				};
				for (key, value) in
					map.iter().filter(|(key, _)| *key != Self::META)
				{
					Self::leaf_paths(key.clone(), value, &mut keys);
				}
				keys
			}
		}
	}

	/// The `meta` table of a tree document, `None` on an env document or
	/// when absent.
	pub fn meta(&self) -> Option<&Map> {
		match self {
			Self::Tree(root) => root.get(Self::META)?.as_map().ok(),
			Self::Env(_) => None,
		}
	}

	/// Set one `meta` entry, creating the table; errors on an env document.
	pub fn set_meta(
		&mut self,
		key: impl Into<SmolStr>,
		value: impl Into<Value>,
	) -> Result<()> {
		match self {
			Self::Tree(root) => {
				Self::table_mut(root, &FieldSegment::key(Self::META))?
					.insert(key, value)?;
				Ok(())
			}
			Self::Env(_) => bevybail!(
				"an env vault carries no metadata: a tree vault (`.toml.age`, \
				`.json.age`) does"
			),
		}
	}

	/// The env document, erroring on a tree with the format named: the
	/// `env` and `exec` verbs work on env vaults only.
	pub fn as_env(&self) -> Result<&EnvDocument> {
		match self {
			Self::Env(doc) => Ok(doc),
			Self::Tree(_) => bevybail!(
				"this is a tree vault, and only an env vault (`.env.age`) \
				becomes an environment"
			),
		}
	}

	/// Merge every entry of `other` in, an existing key kept unless
	/// `replace`. Returns the keys written.
	pub fn merge(
		&mut self,
		other: &Self,
		replace: bool,
	) -> Result<Vec<SmolStr>> {
		match (self, other) {
			(Self::Env(doc), Self::Env(other)) => {
				doc.merge(other, replace).xok()
			}
			(this, other) => {
				let mut written = Vec::new();
				for key in other.keys() {
					if replace || this.get(&key).is_none() {
						this.set(&key, other.get(&key).unwrap_or_default())?;
						written.push(key);
					}
				}
				written.xok()
			}
		}
	}

	/// The text an env line holds for `value`, refusing a nested value.
	fn scalar_text(key: &str, value: &Value) -> Result<SmolStr> {
		match value {
			Value::Str(text) => text.clone(),
			Value::Bool(_)
			| Value::Int(_)
			| Value::Uint(_)
			| Value::Float(_)
			| Value::Null => SmolStr::new(value.to_string()),
			Value::Map(_) | Value::List(_) | Value::Bytes(_) => bevybail!(
				"`{key}` is a {}, and an env vault holds flat `KEY=value` lines: \
				a nested value needs a tree vault (`.toml.age`, `.json.age`)",
				value.kind()
			),
		}
		.xok()
	}

	/// The table at `segment` of `parent`, created when absent.
	fn table_mut<'a>(
		parent: &'a mut Value,
		segment: &FieldSegment,
	) -> Result<&'a mut Value> {
		let key = Self::key_of(segment)?;
		let parent = parent.as_map_mut()?;
		if !parent.contains(&key) {
			parent.insert(key.clone(), Value::map());
		}
		parent
			.get_mut(&key)
			.ok_or_else(|| bevyhow!("table `{key}` vanished on insert"))
	}

	/// A dotted path addresses tables by key; an index is not a vault key.
	fn key_of(segment: &FieldSegment) -> Result<SmolStr> {
		match segment {
			FieldSegment::ObjectKey(key) => key.clone().xok(),
			FieldSegment::ArrayIndex(index) => bevybail!(
				"`[{index}]` indexes a list, and a vault key is a dotted path of \
				table keys"
			),
		}
	}

	/// Push the dotted path of every leaf under `value` onto `keys`.
	fn leaf_paths(prefix: SmolStr, value: &Value, keys: &mut Vec<SmolStr>) {
		match value.as_map() {
			Ok(map) if !map.is_empty() => {
				for (key, value) in map.iter() {
					Self::leaf_paths(
						SmolStr::new(format!("{prefix}.{key}")),
						value,
						keys,
					);
				}
			}
			_ => keys.push(prefix),
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	/// A tree with a meta table and two secrets, as an export writes one.
	fn export() -> VaultDocument {
		let mut doc = VaultDocument::new(VaultFormat::Toml);
		doc.set_meta("app", "mail").unwrap();
		doc.set_meta("exported", "2026-09-17T00:00:00.000Z")
			.unwrap();
		doc.set("secrets.dkim-beetmash-com.value", "-----BEGIN KEY-----")
			.unwrap();
		doc.set("secrets.dkim-beetmash-com.note", "the signing key")
			.unwrap();
		doc.set("secrets.ses-relay.value", "hunter2").unwrap();
		doc
	}

	#[crate::test]
	fn tree_get_set_remove_by_dotted_path() {
		let mut doc = export();
		doc.get("secrets.ses-relay.value")
			.xpect_eq(Some(Value::str("hunter2")));
		doc.get("secrets.missing").xpect_none();
		doc.keys().xpect_eq(vec![
			SmolStr::new("secrets.dkim-beetmash-com.value"),
			SmolStr::new("secrets.dkim-beetmash-com.note"),
			SmolStr::new("secrets.ses-relay.value"),
		]);
		doc.meta()
			.unwrap()
			.get("app")
			.unwrap()
			.xpect_eq(Value::str("mail"));
		doc.remove("secrets.ses-relay.value").xpect_true();
		doc.remove("secrets.ses-relay.value").xpect_false();
		// the emptied table goes with it, its sibling stays
		doc.get("secrets.ses-relay").xpect_none();
		doc.get("secrets.dkim-beetmash-com.note").xpect_some();
	}

	#[crate::test]
	fn toml_render_is_stable() {
		export().render(VaultFormat::Toml).unwrap().xpect_snapshot();
	}

	#[crate::test]
	fn tree_roundtrips_through_both_formats() {
		let doc = export();
		for format in [VaultFormat::Toml, VaultFormat::Json] {
			let text = doc.render(format).unwrap();
			VaultDocument::parse(format, &text)
				.unwrap()
				.xpect_eq(doc.clone());
		}
	}

	#[crate::test]
	fn env_refuses_a_nested_value() {
		let mut doc = VaultDocument::new(VaultFormat::Env);
		doc.set("FLAT", "ok").unwrap();
		doc.set("NESTED", Value::map())
			.unwrap_err()
			.to_string()
			.xpect_contains(".toml.age");
		doc.set_meta("app", "x").unwrap_err();
		export()
			.as_env()
			.unwrap_err()
			.to_string()
			.xpect_contains("env");
	}

	#[crate::test]
	fn env_renders_as_a_tree_and_back() {
		let doc =
			VaultDocument::parse(VaultFormat::Env, "A=1\nB=two\n").unwrap();
		let toml = doc.render(VaultFormat::Toml).unwrap();
		toml.as_str().xpect_contains("A = \"1\"");
		VaultDocument::parse(VaultFormat::Toml, &toml)
			.unwrap()
			.render(VaultFormat::Env)
			.unwrap()
			.xpect_eq("A=1\nB=two\n");
		export()
			.render(VaultFormat::Env)
			.unwrap_err()
			.to_string()
			.xpect_contains("nested");
	}

	#[crate::test]
	fn encrypt_decrypt_roundtrip() {
		let identity = AgeIdentity::generate();
		let mut identities = AgeIdentityFile::default();
		identities.push(identity.clone());
		let doc = export();
		let ciphertext = doc
			.encrypt(VaultFormat::Json, &[identity.to_recipient()])
			.unwrap();
		VaultDocument::decrypt(
			VaultFormat::Json,
			&identities,
			ciphertext.as_bytes(),
		)
		.unwrap()
		.xpect_eq(doc);
	}

	#[crate::test]
	fn merge_keeps_existing_unless_replacing() {
		let mut doc = VaultDocument::new(VaultFormat::Toml);
		doc.set("a.x", "old").unwrap();
		let mut other = VaultDocument::new(VaultFormat::Toml);
		other.set("a.x", "new").unwrap();
		other.set("b", "added").unwrap();
		doc.merge(&other, false)
			.unwrap()
			.xpect_eq(vec![SmolStr::new("b")]);
		doc.get("a.x").xpect_eq(Some(Value::str("old")));
		doc.merge(&other, true).unwrap().len().xpect_eq(2);
		doc.get("a.x").xpect_eq(Some(Value::str("new")));
	}
}
