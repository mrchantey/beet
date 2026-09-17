//! The document format a vault holds, named by its file extension.

use crate::prelude::*;
use core::fmt;

/// The format of a vault's plaintext, read off the extension before `.age`:
/// `.env.age` is flat `KEY=value` lines (the only format the bootstrap
/// loader reads into the process environment), `.toml.age` and `.json.age`
/// are tree documents with room for metadata.
///
/// ## Example
///
/// ```
/// # use beet_core::prelude::*;
/// VaultFormat::from_path(".env.age").unwrap().xpect_eq(VaultFormat::Env);
/// VaultFormat::from_path("infra/secrets/mail--prod.toml.age")
/// 	.unwrap()
/// 	.xpect_eq(VaultFormat::Toml);
/// VaultFormat::from_path("notes.txt.age").unwrap_err();
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub enum VaultFormat {
	/// Flat `KEY=value` lines, the `.env` grammar ([`EnvDocument`]).
	Env,
	/// A TOML tree, the most readable tree format on paper.
	Toml,
	/// A JSON tree.
	Json,
}

impl VaultFormat {
	/// The suffix every vault carries.
	pub const AGE_EXTENSION: &'static str = "age";
	/// Every format, for error messages.
	const ALL: [Self; 3] = [Self::Env, Self::Toml, Self::Json];

	/// The format `path` names, erroring when the path is not `<name>.<format>.age`.
	pub fn from_path(path: impl AsRef<str>) -> Result<Self> {
		let path = path.as_ref();
		let name = path.rsplit('/').next().unwrap_or(path);
		let Some(stem) = name.strip_suffix(".age") else {
			bevybail!(
				"`{path}` is not a vault: a vault ends in `.age` with its \
				format before it, ie `{}`",
				Self::supported()
			);
		};
		let extension = stem.rsplit_once('.').map(|(_, ext)| ext);
		extension
			.and_then(|extension| {
				Self::ALL
					.into_iter()
					.find(|format| format.extension() == extension)
			})
			.ok_or_else(|| {
				bevyhow!(
					"`{path}` names no vault format: the extension before `.age` \
					must be one of `{}`",
					Self::supported()
				)
			})
	}

	/// The format a PLAINTEXT file names by its extension (`.env`, `x.toml`,
	/// `x.json`), what `secrets import` reads.
	pub fn from_plaintext_path(path: impl AsRef<str>) -> Result<Self> {
		let path = path.as_ref();
		let name = path.rsplit('/').next().unwrap_or(path);
		name.rsplit_once('.')
			.map(|(_, extension)| extension)
			.and_then(|extension| {
				Self::ALL
					.into_iter()
					.find(|format| format.extension() == extension)
			})
			.ok_or_else(|| {
				bevyhow!(
					"`{path}` names no vault format: a plaintext file ends in one \
					of `{}`",
					Self::ALL
						.iter()
						.map(|format| format!(".{}", format.extension()))
						.collect::<Vec<_>>()
						.join("`, `")
				)
			})
	}

	/// The extension before `.age`.
	pub fn extension(&self) -> &'static str {
		match self {
			Self::Env => "env",
			Self::Toml => "toml",
			Self::Json => "json",
		}
	}

	/// Whether the format is a tree (carries metadata) rather than flat.
	pub fn is_tree(&self) -> bool { !matches!(self, Self::Env) }

	/// The supported spellings, for error messages.
	fn supported() -> String {
		Self::ALL
			.iter()
			.map(|format| format!(".{}.age", format.extension()))
			.collect::<Vec<_>>()
			.join("`, `")
	}
}

impl fmt::Display for VaultFormat {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(self.extension())
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[crate::test]
	fn from_path_reads_the_extension_before_age() {
		VaultFormat::from_path(".env.age")
			.unwrap()
			.xpect_eq(VaultFormat::Env);
		VaultFormat::from_path("dir/personal.toml.age")
			.unwrap()
			.xpect_eq(VaultFormat::Toml);
		VaultFormat::from_path("export.json.age")
			.unwrap()
			.xpect_eq(VaultFormat::Json);
	}

	#[crate::test]
	fn plaintext_paths_name_their_format() {
		VaultFormat::from_plaintext_path(".env")
			.unwrap()
			.xpect_eq(VaultFormat::Env);
		VaultFormat::from_plaintext_path("dir/export.toml")
			.unwrap()
			.xpect_eq(VaultFormat::Toml);
		VaultFormat::from_plaintext_path("notes.txt").unwrap_err();
	}

	#[crate::test]
	fn rejects_unknown_and_missing_formats() {
		VaultFormat::from_path("foo.age")
			.unwrap_err()
			.to_string()
			.xpect_contains(".toml.age");
		VaultFormat::from_path("foo.yaml.age")
			.unwrap_err()
			.to_string()
			.xpect_contains(".env.age");
		VaultFormat::from_path("foo.toml")
			.unwrap_err()
			.to_string()
			.xpect_contains("ends in `.age`");
	}
}
