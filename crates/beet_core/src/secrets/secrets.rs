//! The `<Secrets>` declaration and the runner's convention for the same
//! file.

use crate::prelude::*;
use std::path::Path;
use std::path::PathBuf;

/// A secrets document an entry declares, `<Secrets path="secrets.toml"/>`:
/// the file at `path` in the nearest ancestor store (the repo store, or a
/// `StoreRef` target's beside it), named by `label` on every document verb's
/// `--document`. A `path` may climb above the store with `..`
/// (`<Secrets path="../secrets.toml"/>` from an entry in a subdirectory of
/// the repo, whose document is the repo's), which a filesystem store
/// re-roots for and a bucket refuses, exactly as `<RepoRoot>` does.
///
/// Loaded twice, by design. The launch reads every top-level unconditional
/// declaration in the repo store out of the entry's prescan, before the
/// entry builds, and sets its `EnvVar` records into the process environment
/// (existing wins), so a declaration constructed in the build walk finds
/// them set; the declaration therefore sits at the entry's top level with no
/// `bx:cfg`, and a lean build keeps it as an inert tag. Then, when the entity
/// is ready, the document is read again, every group the discovered identity
/// opens is verified and the opened document is inserted as [`OpenSecrets`]
/// on the same entity, for a verb reading a non-env record by name. A
/// document with no identity on the machine is one warning and nothing
/// loaded, never an error: a cloud box's repo store never carries one, and a
/// contributor without it still builds. A declaration naming another store
/// (`{StoreRef($cold)}`) is an export target the verbs read and write, never
/// loaded into an environment.
///
/// The loads themselves live beside the store I/O in `beet_net`.
///
/// ```rsx
/// <Secrets/>
/// <Secrets label="mail-prod" path="infra/secrets/mail--prod.toml"/>
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Component, Reflect)]
#[reflect(Component, Default)]
pub struct Secrets {
	/// How a verb names this document, `secrets` by default.
	pub label: SmolStr,
	/// The file within its store, its format named by its extension; a
	/// leading `..` climbs above the store.
	pub path: SmolPath,
}

impl Default for Secrets {
	fn default() -> Self {
		Self {
			label: Self::DEFAULT_LABEL.into(),
			path: SmolPath::new(SecretsDocument::DEFAULT_PATH),
		}
	}
}

impl Secrets {
	/// The label an undeclared or lone document answers to.
	pub const DEFAULT_LABEL: &'static str = "secrets";
	/// The formats a conventional document may be written in, by extension.
	const EXTENSIONS: &'static [&'static str] = &["toml", "json", "ron"];

	/// The document at `path`, labelled by default.
	pub fn new(path: impl AsRef<str>) -> Self {
		Self {
			path: SmolPath::new(path),
			..default()
		}
	}

	/// The declaration with `label`.
	pub fn with_label(mut self, label: impl Into<SmolStr>) -> Self {
		self.label = label.into();
		self
	}

	/// The document's format, from its path.
	pub fn media_type(&self) -> Result<MediaType> {
		SecretsDocument::media_type_of(self.path.as_str())
	}

	/// The test runner's convention: the nearest `secrets.<format>` in the
	/// current directory or its ancestors (the repo's, from any crate's own
	/// directory), found exactly as a `.env` is and loaded after it, its
	/// failure one stderr line since the runner has no logger yet.
	pub fn load_env_vars_nearest() {
		let Some(dir) = Self::nearest_dir() else {
			return;
		};
		if let Err(err) = Self::load_env_vars_from(&dir) {
			crate::cross_log_error!("warning: {err}");
		}
	}

	/// The nearest ancestor of the current directory (itself included)
	/// holding a `secrets.<format>`, `None` when none does.
	pub fn nearest_dir() -> Option<PathBuf> {
		let cwd = fs_ext::current_dir().ok()?;
		cwd.ancestors()
			.find(|dir| matches!(Self::find_in_dir(dir), Ok(Some(_))))
			.map(Path::to_path_buf)
	}

	/// Load the `EnvVar` records of the one `secrets.<format>` in `dir` into
	/// the process environment (existing wins), answering how many landed:
	/// none when there is no such file, an error naming both when there are
	/// two, and an error naming the identity path and the file when there is
	/// no identity to open it with or it opens no group.
	pub fn load_env_vars_from(dir: &Path) -> Result<usize> {
		let Some(path) = Self::find_in_dir(dir)? else {
			return 0.xok();
		};
		let file = path.display();
		let media_type =
			SecretsDocument::media_type_of(&path.to_string_lossy())?;
		let document =
			SecretsDocument::parse(media_type, &fs_ext::read(&path)?)
				.map_err(|err| bevyhow!("`{file}`: {err}"))?;
		let Some(identities) = AgeIdentityFile::discover()? else {
			bevybail!(
				"no age identity at `{}` to open `{file}` with: its {} record(s) \
				were not loaded (`beet vault/keygen` makes one, \
				`vault/restore-identity` restores one)",
				AgeIdentityFile::default_path()?.display(),
				document.record_count()
			);
		};
		let opened = document.open(&identities)?;
		if opened.opened.is_empty() && !document.groups.is_empty() {
			bevybail!(
				"this identity opens no group of `{file}` (locked: {}; pending a \
				`secrets/rekey`: {}): its {} record(s) were not loaded",
				list(&opened.locked),
				list(&opened.pending),
				document.record_count()
			);
		}
		opened.set_env_vars()
	}

	/// The one `secrets.<format>` in `dir` (`secrets.toml`, `secrets.json`,
	/// `secrets.ron`), `None` when there is none, an error naming both when
	/// there are two. Probes the three names rather than listing the
	/// directory: a listing on a js host is a recursive walk, and a repo
	/// root holds a `target/`.
	fn find_in_dir(dir: &Path) -> Result<Option<PathBuf>> {
		let found = Self::EXTENSIONS
			.iter()
			.map(|extension| {
				dir.join(format!("{}.{extension}", SecretsDocument::FILE_STEM))
			})
			.filter(|path| fs_ext::exists(path).unwrap_or(false))
			.collect::<Vec<_>>();
		match found.as_slice() {
			[] => None.xok(),
			[one] => Some(one.clone()).xok(),
			many => bevybail!(
				"{} holds {} secrets documents ({}): the runner loads one by \
				convention, so keep one `secrets.<format>` in a directory",
				dir.display(),
				many.len(),
				many.iter()
					.map(|path| path.display().to_string())
					.collect::<Vec<_>>()
					.join(", ")
			),
		}
	}
}

/// Group names for a message, `none` when empty.
fn list(names: &[SmolStr]) -> String {
	match names.is_empty() {
		true => "none".to_string(),
		false => names.join(", "),
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	/// A temp dir of its own per test.
	#[cfg(not(target_arch = "wasm32"))]
	fn temp_dir(name: &str) -> std::path::PathBuf {
		let dir = std::env::temp_dir()
			.join(format!("beet-secrets-{name}-{}", Timestamp::now().millis()));
		fs_ext::create_dir_all(&dir).unwrap();
		dir
	}

	#[crate::test]
	fn defaults_and_media_type() {
		let secrets = Secrets::default();
		secrets.label.as_str().xpect_eq("secrets");
		secrets.path.as_str().xpect_eq("secrets.toml");
		secrets.media_type().unwrap().xpect_eq(MediaType::Toml);
		Secrets::new("infra/x.json")
			.with_label("mail")
			.media_type()
			.unwrap()
			.xpect_eq(MediaType::Json);
	}

	/// Native only: the document is found on the filesystem and the identity
	/// is set through the environment.
	#[cfg(not(target_arch = "wasm32"))]
	#[crate::test]
	fn loads_env_vars_by_convention() {
		let dir = temp_dir("load");
		// no document: nothing to do
		Secrets::load_env_vars_from(&dir).unwrap().xpect_eq(0);
		let identity = AgeIdentity::generate();
		let mut identities = AgeIdentityFile::default();
		identities.push(identity.clone());
		let mut document = SecretsDocument::default();
		document
			.set(
				&identities,
				"default",
				"BEET_TEST_CONVENTION_VAR",
				"loaded",
				SecretRecord {
					role: Some(SecretRole::EnvVar),
					..default()
				},
			)
			.unwrap();
		document
			.set(
				&identities,
				"default",
				"BEET_TEST_CONVENTION_NOTE",
				"kept",
				default(),
			)
			.unwrap();
		fs_ext::write(dir.join("secrets.toml"), document.to_bytes().unwrap())
			.unwrap();
		// a stray file with the stem is not a document
		fs_ext::write(dir.join("secrets.md"), "# notes").unwrap();

		// no identity anywhere discovery looks: the error names the file and
		// the path
		let previous = env_ext::var(AgeIdentityFile::ENV_VAR).ok();
		let previous_xdg = env_ext::var("XDG_CONFIG_HOME").ok();
		let previous_home = env_ext::var("HOME").ok();
		let no_config = dir.join("no-config").to_string_lossy().to_string();
		// SAFETY: test-only; the discovery test restores the same variables
		unsafe {
			env_ext::remove_var(AgeIdentityFile::ENV_VAR).unwrap();
			env_ext::set_var("XDG_CONFIG_HOME", &no_config).unwrap();
			env_ext::set_var("HOME", &no_config).unwrap();
		}
		let err = Secrets::load_env_vars_from(&dir).unwrap_err().to_string();
		err.as_str()
			.xpect_contains("secrets.toml")
			.xpect_contains("no age identity")
			.xpect_contains("2 record(s)");

		// a stranger's identity opens nothing
		unsafe {
			env_ext::set_var(
				AgeIdentityFile::ENV_VAR,
				&AgeIdentity::generate().to_string(),
			)
			.unwrap();
		}
		Secrets::load_env_vars_from(&dir)
			.unwrap_err()
			.to_string()
			.xpect_contains("opens no group")
			.xpect_contains("locked: default");

		// the identity: the env var lands, the roleless record does not
		unsafe {
			env_ext::set_var(AgeIdentityFile::ENV_VAR, &identity.to_string())
				.unwrap();
		}
		Secrets::load_env_vars_from(&dir).unwrap().xpect_eq(1);
		env_ext::var("BEET_TEST_CONVENTION_VAR")
			.unwrap()
			.xpect_eq("loaded");
		env_ext::var("BEET_TEST_CONVENTION_NOTE").unwrap_err();
		// existing wins: a second load sets nothing
		Secrets::load_env_vars_from(&dir).unwrap().xpect_eq(0);

		// two documents is an error naming both
		fs_ext::write(dir.join("secrets.json"), b"{}").unwrap();
		Secrets::load_env_vars_from(&dir)
			.unwrap_err()
			.to_string()
			.xpect_contains("secrets.json")
			.xpect_contains("secrets.toml");

		unsafe {
			match previous {
				Some(value) => {
					env_ext::set_var(AgeIdentityFile::ENV_VAR, &value)
				}
				None => env_ext::remove_var(AgeIdentityFile::ENV_VAR),
			}
			.unwrap();
			match previous_xdg {
				Some(value) => env_ext::set_var("XDG_CONFIG_HOME", &value),
				None => env_ext::remove_var("XDG_CONFIG_HOME"),
			}
			.unwrap();
			match previous_home {
				Some(value) => env_ext::set_var("HOME", &value),
				None => env_ext::remove_var("HOME"),
			}
			.unwrap();
			env_ext::remove_var("BEET_TEST_CONVENTION_VAR").unwrap();
		}
		fs_ext::remove(&dir).unwrap();
	}
}
