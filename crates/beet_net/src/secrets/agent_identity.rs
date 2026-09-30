//! This machine's own AWS identity, and the session that elevates it.

use crate::prelude::*;
use beet_core::prelude::*;
use std::path::PathBuf;

/// The global document and the one pair it holds: **this machine's own AWS
/// identity**, tier 0c of the credential model.
///
/// It lives at `~/.config/beet/secrets.toml`, stowed there by arch-config, so a
/// fresh machine gets it from one `stow` and one restored age key. It is
/// declared by no entry and nothing auto-loads it: [`AwsExec`] and
/// [`AdminSession`] open it on demand and map the pair onto the sdk's own
/// variable names for one child.
///
/// ## Why the records are not named for the sdk
///
/// A name may live in only one document a launch loads, and every repo's own
/// document already holds `AWS_ACCESS_KEY_ID`. Naming these the same would mean
/// a launch in any repo either shadowing its deployer with this read-only pair
/// or refusing to load. So they are named for whose they are, and the mapping
/// onto the sdk's names happens per child, where it is visible.
pub struct AgentIdentity;

impl AgentIdentity {
	/// The record holding `beet-agent`'s access key id.
	pub const KEY_ID: &'static str = "BEET_AGENT_ACCESS_KEY_ID";
	/// The record holding its secret half.
	pub const KEY_SECRET: &'static str = "BEET_AGENT_SECRET_ACCESS_KEY";
	/// The iam user this pair belongs to, which also names its MFA device, so
	/// the device's arn is `mfa/beet-agent`.
	pub const USER: &'static str = "beet-agent";
	/// The role [`AdministratorAccess`](AdminSession) is attached to. Reachable
	/// from [`USER`](Self::USER) alone, and only with an MFA code.
	pub const ADMIN_ROLE: &'static str = "beet-admin";

	/// `~/.config/beet/secrets.toml`, beside the age identity that opens it.
	/// [`fs_ext::config_dir`] is `~/.config`, so the `beet` segment is ours to
	/// add, exactly as [`AgeIdentityFile`]'s own path constant does.
	pub fn path() -> Result<PathBuf> {
		fs_ext::config_dir().map(|dir| dir.join("beet").join("secrets.toml"))
	}

	/// The document, or the one instruction that produces it. Absence is a
	/// machine that has not been set up rather than an error to decode, so it
	/// says what to run.
	pub async fn handle() -> Result<SecretsHandle> {
		let path = Self::path()?;
		if !fs_ext::exists(&path)? {
			bevybail!(
				"no global document at `{}`, so this machine has no aws \
				identity of its own: clone arch-config and `just \
				stow-symlinks` puts one there, then restore the age key that \
				opens it",
				path.display()
			);
		}
		SecretsHandle::from_uri(&path.to_string_lossy())
	}

	/// `beet-agent`'s pair under the SDK's own names, ready for a child.
	///
	/// Reads the two records by name through the same narrowing
	/// [`SecretsExec`] uses, so a document missing one half says which rather
	/// than handing a child half a credential.
	pub async fn sdk_pair() -> Result<Vec<(SmolStr, SmolStr)>> {
		let handle = Self::handle().await?;
		let pairs = SecretsExec::narrow(
			handle
				.read()
				.await?
				.open(&AgeIdentityFile::require()?)?
				.env_vars(),
			Some(&format!("{},{}", Self::KEY_ID, Self::KEY_SECRET)),
			&handle.describe(),
		)?;
		pairs
			.into_iter()
			.map(|(name, value)| match name.as_str() {
				Self::KEY_ID => ("AWS_ACCESS_KEY_ID".into(), value),
				_ => ("AWS_SECRET_ACCESS_KEY".into(), value),
			})
			.collect::<Vec<_>>()
			.xok()
	}
}

/// What [`AdminSession::read`] found on tmpfs.
#[derive(Debug, Clone)]
pub enum SessionState {
	/// A session with time left on it.
	Live(AdminSession),
	/// One that has lapsed, and whose file has just been removed. Worth
	/// distinguishing from [`None`](Self::None) so a caller can say why the
	/// identity changed under it.
	Expired,
	/// No session, which is the normal state.
	None,
}

/// A live administrator session: the temporary credentials
/// `sts:AssumeRole` minted, on tmpfs and nowhere else.
///
/// Stored under `$XDG_RUNTIME_DIR/beet/admin-session`, which is tmpfs on every
/// Linux desktop, so the session dies with the login session at the latest and
/// with its own expiry otherwise. Deliberately not the config directory: an
/// administrator credential that survives a reboot is a long-lived
/// administrator credential, which is the thing this whole model removes.
#[derive(Debug, Clone)]
pub struct AdminSession {
	/// The temporary access key id.
	pub key_id: SmolStr,
	/// Its secret half.
	pub secret: SmolStr,
	/// The session token, which the sdk requires beside the pair.
	pub token: SmolStr,
	/// When the grant lapses, as a unix second.
	pub expires: i64,
}

impl AdminSession {
	/// Where the session is written. `$XDG_RUNTIME_DIR` when set, else a
	/// per-uid path under `/run/user`, which is what the variable would have
	/// named anyway.
	pub fn path() -> Result<PathBuf> {
		let dir = match env_ext::var("XDG_RUNTIME_DIR") {
			Ok(dir) if !dir.is_empty() => PathBuf::from(dir.as_str()),
			_ => bevybail!(
				"no `XDG_RUNTIME_DIR`, so there is no tmpfs to keep an \
				administrator session on, and it will not be written \
				anywhere that survives a reboot: run the command under `-- \
				<command>` instead, which keeps nothing"
			),
		};
		dir.join("beet").join("admin-session").xok()
	}

	/// What is on tmpfs: a live session, one that has lapsed, or nothing.
	///
	/// A lapsed session is distinguished from none deliberately, and its file is
	/// removed either way. "Your session ran out" and "you never had one" have
	/// the same remedy but very different causes, and a caller that cannot tell
	/// them apart cannot explain why a command that worked a minute ago stopped.
	pub fn read() -> Result<SessionState> {
		let path = Self::path()?;
		if !fs_ext::exists(&path)? {
			return SessionState::None.xok();
		}
		let body = fs_ext::read_to_string(&path)?;
		let field = |key: &str| {
			body.lines()
				.find_map(|line| line.strip_prefix(&format!("{key}=")))
				.map(str::trim)
		};
		let (Some(key_id), Some(secret), Some(token), Some(expires)) = (
			field("AWS_ACCESS_KEY_ID"),
			field("AWS_SECRET_ACCESS_KEY"),
			field("AWS_SESSION_TOKEN"),
			field("BEET_ADMIN_EXPIRES"),
		) else {
			// a truncated file is not a session, and keeping it would make
			// every later read fail the same way
			fs_ext::remove(&path)?;
			return SessionState::None.xok();
		};
		let session = Self {
			key_id: key_id.into(),
			secret: secret.into(),
			token: token.into(),
			expires: expires.parse::<i64>().unwrap_or_default(),
		};
		match session.remaining_secs() > 0 {
			true => SessionState::Live(session).xok(),
			false => {
				fs_ext::remove(&path)?;
				SessionState::Expired.xok()
			}
		}
	}

	/// Seconds left on the grant, zero once it has lapsed.
	pub fn remaining_secs(&self) -> i64 {
		(self.expires - Timestamp::now().secs()).max(0)
	}

	/// Write the session to tmpfs, mode 600 before any value reaches it, so
	/// the credential is never briefly world readable.
	pub fn write(&self) -> Result {
		let path = Self::path()?;
		if let Some(parent) = path.parent() {
			fs_ext::create_dir_all(parent)?;
		}
		fs_ext::write(&path, "")?;
		Self::restrict(&path)?;
		fs_ext::write(
			&path,
			format!(
				"AWS_ACCESS_KEY_ID={}\nAWS_SECRET_ACCESS_KEY={}\n\
				 AWS_SESSION_TOKEN={}\nBEET_ADMIN_EXPIRES={}\n",
				self.key_id, self.secret, self.token, self.expires
			),
		)?;
		Self::restrict(&path)
	}

	/// The three variables a child needs to act as the role.
	pub fn sdk_vars(&self) -> Vec<(SmolStr, SmolStr)> {
		vec![
			("AWS_ACCESS_KEY_ID".into(), self.key_id.clone()),
			("AWS_SECRET_ACCESS_KEY".into(), self.secret.clone()),
			("AWS_SESSION_TOKEN".into(), self.token.clone()),
		]
	}

	/// Owner-only, or nothing: a session file another user can read is not a
	/// session file.
	fn restrict(path: &std::path::Path) -> Result {
		cfg_if! {
			if #[cfg(unix)] {
				use std::os::unix::fs::PermissionsExt;
				std::fs::set_permissions(
					path,
					std::fs::Permissions::from_mode(0o600),
				)?;
				OK
			} else {
				let _ = path;
				bevybail!(
					"this platform cannot restrict a file to its owner, so an \
					administrator session will not be written: run the \
					command under `-- <command>` instead"
				)
			}
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use beet_core::prelude::*;

	/// The record names are not the sdk's, because a name lives in one
	/// document and every repo's own already holds those.
	#[beet_core::test]
	fn the_records_are_not_named_for_the_sdk() {
		AgentIdentity::KEY_ID.xpect_contains("BEET_AGENT");
		AgentIdentity::KEY_SECRET.xpect_contains("BEET_AGENT");
		// the collision this naming exists to avoid
		(AgentIdentity::KEY_ID != "AWS_ACCESS_KEY_ID").xpect_true();
		(AgentIdentity::KEY_SECRET != "AWS_SECRET_ACCESS_KEY").xpect_true();
	}

	/// The document is under `beet/`, not loose in `~/.config`.
	/// `fs_ext::config_dir` is the XDG directory rather than beet's own, and
	/// getting that wrong reports "no global document" on a machine that has
	/// one -- which reads as a setup problem rather than as a path bug.
	#[beet_core::test]
	fn the_document_is_under_the_beet_config_dir() {
		let path = AgentIdentity::path().unwrap();
		path.to_string_lossy()
			.as_ref()
			.xpect_contains("beet/secrets.toml");
		path.parent()
			.unwrap()
			.file_name()
			.unwrap()
			.to_string_lossy()
			.as_ref()
			.xpect_eq("beet");
	}

	/// A lapsed session reads as no session at all, and its file goes with it:
	/// a stale credential on disk is a trap for the next reader.
	#[beet_core::test]
	fn a_lapsed_session_is_no_session() {
		let past = AdminSession {
			key_id: "AKIAEXAMPLE".into(),
			secret: "x".into(),
			token: "y".into(),
			expires: Timestamp::now().secs() - 1,
		};
		past.remaining_secs().xpect_eq(0);
		let future = AdminSession {
			expires: Timestamp::now().secs() + 600,
			..past.clone()
		};
		(future.remaining_secs() > 590).xpect_true();
		// the three the sdk needs, token included: a role's credentials
		// without it authenticate as nobody
		future
			.sdk_vars()
			.iter()
			.map(|(name, _)| name.to_string())
			.collect::<Vec<_>>()
			.xpect_eq(vec![
				"AWS_ACCESS_KEY_ID",
				"AWS_SECRET_ACCESS_KEY",
				"AWS_SESSION_TOKEN",
			]);
	}
}
