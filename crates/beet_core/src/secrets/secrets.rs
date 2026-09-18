//! The `<Secrets>` declaration and the runner's convention for the same
//! file.

use crate::prelude::*;
use std::path::Path;
use std::path::PathBuf;

/// A secrets document an entry declares, `<Secrets path="secrets.toml.age"/>`:
/// the file at `path` in the nearest ancestor store (the repo store, or a
/// `StoreRef` target's beside it), named by `label` on every document verb's
/// `--vault`. When the entity is ready its document is read, every group the
/// discovered identity opens is verified, its `EnvVar` records land in the
/// process environment (existing wins) and the opened document is inserted
/// as [`OpenSecrets`] on the same entity. A document with no identity on the
/// machine is one warning and nothing loaded, never an error: a cloud box's
/// repo store never carries one, and a contributor without it still builds.
///
/// The load itself lives beside the store I/O in `beet_net`; this is the
/// declaration a lean build keeps as an inert tag.
///
/// ```rsx
/// <Secrets/>
/// <Secrets label="mail-prod" path="infra/secrets/mail--prod.toml.age"/>
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Component, Reflect)]
#[reflect(Component, Default)]
pub struct Secrets {
	/// How a verb names this document, `secrets` by default.
	pub label: SmolStr,
	/// The file within its store, its format named by the extension before
	/// `.age`.
	pub path: RelPath,
}

impl Default for Secrets {
	fn default() -> Self {
		Self {
			label: Self::DEFAULT_LABEL.into(),
			path: RelPath::new(SecretsDocument::DEFAULT_PATH),
		}
	}
}

impl Secrets {
	/// The label an undeclared or lone document answers to.
	pub const DEFAULT_LABEL: &'static str = "secrets";

	/// The document at `path`, labelled by default.
	pub fn new(path: impl AsRef<str>) -> Self {
		Self {
			path: RelPath::new(path),
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

	/// The test runner's convention: the document beside the `.env` it
	/// loaded (or in the cwd when there was none), loaded exactly as `.env`
	/// is, its failure one stderr line since the runner has no logger yet.
	pub fn load_env_vars_beside_dotenv() {
		let Some(dir) =
			env_ext::dotenv_dir().or_else(|| fs_ext::current_dir().ok())
		else {
			return;
		};
		if let Err(err) = Self::load_env_vars_from(&dir) {
			crate::cross_log_error!("warning: {err}");
		}
	}

	/// Load the `EnvVar` records of the one `secrets.*.age` in `dir` into
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
				were not loaded (`beet secrets/keygen` makes one, \
				`secrets/restore-identity` restores one)",
				AgeIdentityFile::default_path()?.display(),
				document.secrets.len()
			);
		};
		let opened = document.open(&identities)?;
		if opened.opened.is_empty() && !document.groups.is_empty() {
			bevybail!(
				"this identity opens no group of `{file}` (locked: {}; pending a \
				`secrets/rekey`: {}): its {} record(s) were not loaded",
				list(&opened.locked),
				list(&opened.pending),
				document.secrets.len()
			);
		}
		opened.set_env_vars()
	}

	/// The one `secrets.*.age` in `dir`, `None` when there is none, an error
	/// naming both when there are two.
	pub fn find_in_dir(dir: &Path) -> Result<Option<PathBuf>> {
		if !fs_ext::exists(dir)? {
			return None.xok();
		}
		let mut found = ReadDir::files(dir)?
			.into_iter()
			.filter(|path| {
				path.file_name().and_then(|name| name.to_str()).is_some_and(
					|name| {
						name.starts_with(SecretsDocument::FILE_PREFIX)
							&& name.ends_with(SecretsDocument::SUFFIX)
					},
				)
			})
			.collect::<Vec<_>>();
		found.sort();
		match found.as_slice() {
			[] => None.xok(),
			[one] => Some(one.clone()).xok(),
			many => bevybail!(
				"{} holds {} secrets documents ({}): the runner loads one by \
				convention, so keep one `secrets.*.age` beside `.env`",
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
		secrets.path.as_str().xpect_eq("secrets.toml.age");
		secrets.media_type().unwrap().xpect_eq(MediaType::Toml);
		Secrets::new("infra/x.json.age")
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
				"BEET_TEST_CONVENTION_VAR",
				"loaded",
				SecretRecord {
					role: Some(SecretRole::EnvVar),
					..default()
				},
			)
			.unwrap();
		document
			.set(&identities, "BEET_TEST_CONVENTION_NOTE", "kept", default())
			.unwrap();
		fs_ext::write(
			dir.join("secrets.toml.age"),
			document.to_bytes().unwrap(),
		)
		.unwrap();

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
			.xpect_contains("secrets.toml.age")
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
		fs_ext::write(dir.join("secrets.json.age"), b"{}").unwrap();
		Secrets::load_env_vars_from(&dir)
			.unwrap_err()
			.to_string()
			.xpect_contains("secrets.json.age")
			.xpect_contains("secrets.toml.age");

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
