//! The `keys.txt` identity file and where a process finds it.

use crate::prelude::*;
use core::fmt;
use std::path::Path;
use std::path::PathBuf;

/// The identity file, `keys.txt`: one identity per line, blank lines and `#`
/// comments preserved, the format `age-keygen` writes and sops reads. One per
/// human, in the OS config (`~/.config/beet/age/keys.txt`), holding every
/// identity they have (a second line for a hardware key), and found by
/// [`discover`](Self::discover) so nothing is ever passed in.
///
/// [`Display`] is the file's text, secrets included, for the writers;
/// [`Debug`] redacts.
///
/// ## Example
///
/// ```
/// # use beet_core::prelude::*;
/// let mut file = AgeIdentityFile::default();
/// file.push(AgeIdentity::generate());
/// let text = file.to_string();
/// AgeIdentityFile::parse(&text)
/// 	.unwrap()
/// 	.recipients()
/// 	.xpect_eq(file.recipients());
/// ```
#[derive(Default, Clone)]
pub struct AgeIdentityFile {
	lines: Vec<Line>,
}

/// One line of the file, verbatim text or a parsed identity.
#[derive(Clone)]
enum Line {
	/// A comment or a blank line, kept as written.
	Text(String),
	Identity(AgeIdentity),
}

impl AgeIdentityFile {
	/// The one environment variable this subsystem adds, for a CI runner or a
	/// container that deploys, where the identity arrives from the runner's
	/// secret store and there is no home directory worth writing to. Holds a
	/// path to an identity file, or the identity itself (told by
	/// [`AgeIdentity::PREFIX`]). A local machine never sets it and lives off
	/// the file; a lambda or a cloud box has neither.
	///
	/// NOT a `BootstrapConfig` knob: a knob renders onto argv and env for
	/// child launches, and an identity on an argv line is the one place it
	/// must never be.
	pub const ENV_VAR: &'static str = "BEET_AGE_IDENTITY";

	/// The path `keys.txt` takes below a config directory.
	const CONFIG_PATH: &'static str = "beet/age/keys.txt";
	/// sops' identity file, read as a fallback so a sops user's identity
	/// works unmoved.
	const SOPS_CONFIG_PATH: &'static str = "sops/age/keys.txt";

	/// Parse the `keys.txt` grammar. A line that is neither blank, a comment
	/// nor an identity is an error naming its line number, never its content.
	pub fn parse(contents: &str) -> Result<Self> {
		contents
			.lines()
			.enumerate()
			.map(|(index, line)| {
				let trimmed = line.trim();
				if trimmed.is_empty() || trimmed.starts_with('#') {
					return Line::Text(line.to_string()).xok();
				}
				trimmed.parse::<AgeIdentity>().map(Line::Identity).map_err(
					|err| bevyhow!("identity file line {}: {err}", index + 1),
				)
			})
			.collect::<Result<Vec<_>>>()
			.map(|lines| Self { lines })
	}

	/// Read and parse the file at `path`.
	pub fn read(path: impl AsRef<Path>) -> Result<Self> {
		let path = path.as_ref();
		fs_ext::read_to_string(path)?
			.xmap(|contents| Self::parse(&contents))
			.map_err(|err| bevyhow!("{}: {err}", path.display()))
	}

	/// Write the file to `path`, owner-readable only and its directory
	/// created ([`fs_ext::write_private`]).
	pub fn write(&self, path: impl AsRef<Path>) -> Result<()> {
		fs_ext::write_private(path, self.to_string())?.xok()
	}

	/// Every identity in the file, in order.
	pub fn identities(&self) -> impl Iterator<Item = &AgeIdentity> {
		self.lines.iter().filter_map(|line| match line {
			Line::Identity(identity) => Some(identity),
			Line::Text(_) => None,
		})
	}

	/// The public half of every identity, in order.
	pub fn recipients(&self) -> Vec<AgeRecipient> {
		self.identities().map(AgeIdentity::to_recipient).collect()
	}

	/// The number of identities.
	pub fn len(&self) -> usize { self.identities().count() }

	/// Whether the file holds no identity.
	pub fn is_empty(&self) -> bool { self.len() == 0 }

	/// Append an identity the way `age-keygen` writes one: a `# created:`
	/// line, a `# public key:` line, then the identity, separated from what
	/// precedes it by a blank line.
	pub fn push(&mut self, identity: AgeIdentity) {
		if self.lines.last().is_some_and(|line| !line.is_blank()) {
			self.lines.push(Line::Text(String::new()));
		}
		if let Ok(now) = Timestamp::try_now() {
			self.lines.push(Line::Text(format!(
				"# created: {}",
				now.format_iso8601()
			)));
		}
		self.lines.push(Line::Text(format!(
			"# public key: {}",
			identity.to_recipient()
		)));
		self.lines.push(Line::Identity(identity));
	}

	/// Decrypt an age file with whichever identity here it names, see
	/// [`AgeIdentity::decrypt_any`]. Errors on an empty file.
	pub fn decrypt(&self, ciphertext: &[u8]) -> Result<Vec<u8>> {
		if self.is_empty() {
			bevybail!("the identity file holds no identity");
		}
		AgeIdentity::decrypt_any(self.identities(), ciphertext)
	}

	/// Where a new identity is written: `keys.txt` under
	/// [`fs_ext::config_dir`].
	pub fn default_path() -> Result<PathBuf> {
		fs_ext::config_dir().map(|dir| dir.join(Self::CONFIG_PATH))
	}

	/// The identity file this process uses, environment first then files:
	///
	/// 1. [`ENV_VAR`](Self::ENV_VAR) when set, a path or an inline identity
	/// 2. `$XDG_CONFIG_HOME/beet/age/keys.txt`
	/// 3. `~/.config/beet/age/keys.txt`
	/// 4. sops' `~/.config/sops/age/keys.txt`
	///
	/// `Ok(None)` when none of these exists, which is a cloud box's normal
	/// state and a contributor's before `keygen`; an error when one exists
	/// and does not parse, or the variable names a missing path.
	pub fn discover() -> Result<Option<Self>> {
		if let Ok(value) = env_ext::var(Self::ENV_VAR) {
			let value = value.trim();
			return match value.starts_with(AgeIdentity::PREFIX) {
				true => Self::parse(value),
				false => Self::read(value),
			}
			.map(Some);
		}
		for path in Self::candidate_paths() {
			if fs_ext::exists(&path)? {
				return Self::read(path).map(Some);
			}
		}
		None.xok()
	}

	/// [`discover`](Self::discover) for a caller that cannot do without an
	/// identity: absence is an error naming how one is made or restored, the
	/// one message every vault verb shares.
	pub fn require() -> Result<Self> {
		Self::discover()?.ok_or_else(|| {
			bevyhow!(
				"no age identity: `beet vault/keygen` makes one at `{}`, \
				`vault/restore-identity --file=<backup>` restores a backup, \
				and a CI runner passes one through `{}`",
				Self::default_path()
					.map(|path| path.display().to_string())
					.unwrap_or_else(|_| Self::CONFIG_PATH.to_string()),
				Self::ENV_VAR
			)
		})
	}

	/// The files [`discover`](Self::discover) checks after the environment,
	/// in order, the beet file before sops' in each config directory.
	fn candidate_paths() -> Vec<PathBuf> {
		let mut dirs = [
			fs_ext::config_dir().ok(),
			fs_ext::home_dir().ok().map(|home| home.join(".config")),
		]
		.into_iter()
		.flatten()
		.collect::<Vec<_>>();
		dirs.dedup();
		dirs.iter()
			.flat_map(|dir| {
				[
					dir.join(Self::CONFIG_PATH),
					dir.join(Self::SOPS_CONFIG_PATH),
				]
			})
			.collect()
	}
}

impl Line {
	fn is_blank(&self) -> bool {
		matches!(self, Line::Text(text) if text.trim().is_empty())
	}
}

/// The file's text, one line each with a trailing newline, comments as
/// written and every identity as its secret key line.
impl fmt::Display for AgeIdentityFile {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		for line in &self.lines {
			match line {
				Line::Text(text) => writeln!(f, "{text}")?,
				Line::Identity(identity) => writeln!(f, "{identity}")?,
			}
		}
		Ok(())
	}
}

/// Redacted: the count only.
impl fmt::Debug for AgeIdentityFile {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "AgeIdentityFile({} identities, <redacted>)", self.len())
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[crate::test]
	fn parse_preserves_comments_and_blanks() {
		let identity = AgeIdentity::generate();
		let text = format!(
			"# created: 2026-09-15T00:00:00.000Z\n# public key: {}\n{identity}\n\n# a trailing note\n",
			identity.to_recipient()
		);
		let file = AgeIdentityFile::parse(&text).unwrap();
		file.len().xpect_eq(1);
		file.to_string().xpect_eq(text);
	}

	#[crate::test]
	fn push_then_identities_roundtrip() {
		let alice = AgeIdentity::generate();
		let bob = AgeIdentity::generate();
		let mut file = AgeIdentityFile::default();
		file.push(alice.clone());
		file.push(bob.clone());
		file.identities()
			.cloned()
			.collect::<Vec<_>>()
			.xpect_eq(vec![alice.clone(), bob.clone()]);
		let text = file.to_string();
		text.as_str()
			.xpect_contains(format!("# public key: {}", bob.to_recipient()));
		AgeIdentityFile::parse(&text)
			.unwrap()
			.recipients()
			.xpect_eq(vec![alice.to_recipient(), bob.to_recipient()]);
	}

	#[crate::test]
	fn decrypt_tries_every_identity() {
		let alice = AgeIdentity::generate();
		let bob = AgeIdentity::generate();
		let mut file = AgeIdentityFile::default();
		file.push(alice);
		file.push(bob.clone());
		let ciphertext =
			AgeRecipient::encrypt(&[bob.to_recipient()], b"for bob").unwrap();
		file.decrypt(ciphertext.as_bytes())
			.unwrap()
			.xpect_eq(b"for bob".to_vec());
		AgeIdentityFile::default()
			.decrypt(ciphertext.as_bytes())
			.unwrap_err()
			.to_string()
			.xpect_contains("no identity");
	}

	#[crate::test]
	fn rejects_a_non_identity_line_by_number() {
		AgeIdentityFile::parse("# fine\n\nnot-a-key\n")
			.unwrap_err()
			.to_string()
			.xpect_contains("line 3")
			.xnot()
			.xpect_contains("not-a-key");
	}

	#[crate::test]
	fn debug_redacts() {
		let mut file = AgeIdentityFile::default();
		file.push(AgeIdentity::generate());
		format!("{file:?}")
			.xpect_eq("AgeIdentityFile(1 identities, <redacted>)")
			.xnot()
			.xpect_contains(AgeIdentity::PREFIX);
	}

	/// Native only: the browser host has no mutable environment. One test
	/// for every discovery step, since they share the process environment.
	#[cfg(not(target_arch = "wasm32"))]
	#[crate::test]
	fn discover_prefers_env_then_config_files() {
		let dir = std::env::temp_dir()
			.join(format!("beet-age-{}", Timestamp::now().millis()));
		let previous_xdg = env_ext::var("XDG_CONFIG_HOME").ok();
		// SAFETY: test-only; no other test reads these variables
		unsafe {
			env_ext::set_var("XDG_CONFIG_HOME", &dir.to_string_lossy())
				.unwrap();
		}

		// the config file
		let on_disk = AgeIdentity::generate();
		let mut file = AgeIdentityFile::default();
		file.push(on_disk.clone());
		let path = AgeIdentityFile::default_path().unwrap();
		path.xpect_eq(dir.join("beet/age/keys.txt"));
		file.write(&path).unwrap();
		#[cfg(unix)]
		{
			use std::os::unix::fs::PermissionsExt;
			(std::fs::metadata(&path).unwrap().permissions().mode() & 0o777)
				.xpect_eq(0o600);
		}
		AgeIdentityFile::discover()
			.unwrap()
			.unwrap()
			.recipients()
			.xpect_eq(vec![on_disk.to_recipient()]);

		// the inline env var wins over the file
		let inline = AgeIdentity::generate();
		unsafe {
			env_ext::set_var(AgeIdentityFile::ENV_VAR, &inline.to_string())
				.unwrap();
		}
		AgeIdentityFile::discover()
			.unwrap()
			.unwrap()
			.recipients()
			.xpect_eq(vec![inline.to_recipient()]);

		// the env var as a path
		let elsewhere = dir.join("elsewhere.txt");
		let mut file = AgeIdentityFile::default();
		file.push(inline.clone());
		file.push(on_disk.clone());
		file.write(&elsewhere).unwrap();
		unsafe {
			env_ext::set_var(
				AgeIdentityFile::ENV_VAR,
				&elsewhere.to_string_lossy(),
			)
			.unwrap();
		}
		AgeIdentityFile::discover()
			.unwrap()
			.unwrap()
			.len()
			.xpect_eq(2);

		// a named path must exist
		unsafe {
			env_ext::set_var(
				AgeIdentityFile::ENV_VAR,
				&dir.join("missing.txt").to_string_lossy(),
			)
			.unwrap();
		}
		AgeIdentityFile::discover().unwrap_err();

		// restore
		unsafe {
			env_ext::remove_var(AgeIdentityFile::ENV_VAR).unwrap();
			match previous_xdg {
				Some(value) => env_ext::set_var("XDG_CONFIG_HOME", &value),
				None => env_ext::remove_var("XDG_CONFIG_HOME"),
			}
			.unwrap();
		}
		fs_ext::remove(&dir).unwrap();
	}
}
