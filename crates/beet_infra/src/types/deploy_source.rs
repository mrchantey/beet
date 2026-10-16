//! The source tree a deploy was built from.
use beet_core::prelude::*;

/// The source tree a deploy was built from, recorded on its
/// [`ArtifactLedger`] so a released version names the code it runs.
///
/// A deploy builds from the working tree rather than from a commit, so the
/// commit alone does not pin what shipped: a dirty tree also records a digest
/// of its uncommitted changes.
///
/// Read best-effort by [`read`](Self::read): a launch outside a git checkout,
/// with no `git` on its path, or in a repository with no commit records
/// [`Unknown`](Self::Unknown) and deploys anyway. A detached or shallow
/// checkout still names its commit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DeploySource {
	/// Built from a git checkout.
	Git {
		/// The checked out commit, `git rev-parse HEAD`.
		commit: SmolStr,
		/// `git describe --always --dirty`, ie `v0.1.0-3-g38f6913-dirty`. Its
		/// `-dirty` reflects tracked changes alone, as git defines it.
		describe: SmolStr,
		/// A sha256 over the uncommitted changes, tracked changes and
		/// untracked non-ignored files alike, `None` for a clean tree. The same
		/// tree always hashes the same, so two deploys carrying one diff record
		/// one hash.
		#[serde(default)]
		diff_hash: Option<SmolStr>,
	},
	/// The source could not be read.
	Unknown {
		/// Why, ie git's own explanation.
		reason: SmolStr,
	},
}

/// A ledger published before the source was recorded.
impl Default for DeploySource {
	fn default() -> Self { Self::unknown("not recorded") }
}

impl core::fmt::Display for DeploySource {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		match self {
			Self::Git {
				commit,
				describe,
				diff_hash: None,
			} => write!(f, "{describe} ({commit})"),
			Self::Git {
				commit,
				describe,
				diff_hash: Some(diff_hash),
			} => {
				write!(f, "{describe} ({commit}, uncommitted diff {diff_hash})")
			}
			Self::Unknown { reason } => write!(f, "unknown source ({reason})"),
		}
	}
}

impl DeploySource {
	fn unknown(reason: impl Into<SmolStr>) -> Self {
		Self::Unknown {
			reason: reason.into(),
		}
	}
}

#[cfg(not(target_arch = "wasm32"))]
impl DeploySource {
	/// The source of the checkout holding `dir`, never failing: whatever git
	/// cannot answer is [`Unknown`](Self::Unknown), so a deploy from outside a
	/// checkout still ships.
	pub async fn read(dir: &AbsPath) -> Self {
		Self::read_git(dir).await.unwrap_or_else(Self::unknown)
	}

	async fn read_git(dir: &AbsPath) -> Result<Self, SmolStr> {
		// every later read runs at the top, since `ls-files` lists only below
		// its working directory where `diff` covers the whole tree
		let root = Self::git(dir, &["rev-parse", "--show-toplevel"])
			.await?
			.xmap(Self::line)
			.xmap(AbsPath::new)
			.map_err(|err| SmolStr::from(err.to_string()))?;
		let commit = Self::git(&root, &["rev-parse", "--verify", "-q", "HEAD"])
			.await
			.map_err(|_| SmolStr::new("the repository has no commit"))?
			.xmap(Self::line);
		let describe = Self::git(&root, &["describe", "--always", "--dirty"])
			.await?
			.xmap(Self::line);
		Self::Git {
			commit: commit.into(),
			describe: describe.into(),
			diff_hash: Self::diff_hash(&root).await?,
		}
		.xok()
	}

	/// The sha256 of every uncommitted change under `root`, `None` when there
	/// is none: the binary diff of the tracked files against `HEAD`, staged or
	/// not, then each untracked non-ignored file's path and content digest.
	async fn diff_hash(root: &AbsPath) -> Result<Option<SmolStr>, SmolStr> {
		use sha2::Digest;
		// plain output: no color, external diff or textconv a user's config
		// could swap in, and `--binary` so a binary edit moves the hash
		let diff = Self::git(root, &[
			"diff",
			"HEAD",
			"--binary",
			"--no-color",
			"--no-ext-diff",
			"--no-textconv",
		])
		.await?;
		let untracked = Self::git(root, &[
			"ls-files",
			"--others",
			"--exclude-standard",
			"-z",
		])
		.await?;
		if diff.is_empty() && untracked.is_empty() {
			return Ok(None);
		}
		let mut hasher = sha2::Sha256::new();
		hasher.update(diff.len().to_le_bytes());
		hasher.update(&diff);
		for path in untracked
			.split(|byte| *byte == 0)
			.filter(|path| !path.is_empty())
		{
			// a path that will not read (a nested repository, a dangling
			// symlink) still hashes the same every time
			let content = String::from_utf8_lossy(path)
				.xmap(|path| root.join(path))
				.xmap(|path| digest_ext::hex_file::<sha2::Sha256>(path))
				.unwrap_or_else(|_| "unreadable".into());
			hasher.update(path);
			hasher.update([0]);
			hasher.update(content.as_bytes());
			hasher.update([0]);
		}
		digest_ext::to_hex(hasher.finalize().as_slice())
			.xmap(SmolStr::from)
			.xmap(Some)
			.xok()
	}

	/// Run git in `dir`: its stdout, else the reason it gave, ie
	/// `not a git repository (or any of the parent directories): .git`.
	async fn git(dir: &AbsPath, args: &[&str]) -> Result<Vec<u8>, SmolStr> {
		let output = ChildProcess::new("git")
			.with_args(args.iter().copied())
			.with_cwd(dir.clone())
			.into_command_async()
			.output()
			.await
			.map_err(|err| match err.kind() {
				std::io::ErrorKind::NotFound => {
					SmolStr::new("git is not installed")
				}
				_ => format!("cannot run git: {err}").into(),
			})?;
		if output.status.success() {
			return Ok(output.stdout);
		}
		String::from_utf8_lossy(&output.stderr)
			.lines()
			.map(str::trim)
			.find(|line| !line.is_empty())
			.map(|line| line.trim_start_matches("fatal: "))
			.unwrap_or("git failed")
			.xmap(SmolStr::from)
			.xmap(Err)
	}

	/// A one-line answer, trimmed of its newline.
	fn line(stdout: Vec<u8>) -> String {
		String::from_utf8_lossy(&stdout).trim().to_string()
	}
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
	use super::*;

	/// Run git in `dir`, failing the test on a non-zero exit. Commits name
	/// their author and skip signing, so the user's own config cannot fail
	/// one.
	fn git(dir: &AbsPath, args: &[&str]) -> String {
		ChildProcess::new("git")
			.with_args(
				[
					"-c",
					"user.name=beet",
					"-c",
					"user.email=beet@example.com",
					"-c",
					"commit.gpgsign=false",
				]
				.into_iter()
				.chain(args.iter().copied()),
			)
			.with_cwd(dir.clone())
			.run_stdout()
			.unwrap()
			.trim()
			.to_string()
	}

	/// A repository in the system temp dir, outside every other checkout, with
	/// one commit.
	fn committed_repo() -> TempDir {
		let dir = TempDir::new().unwrap();
		git(&dir, &["init", "-q"]);
		fs_ext::write(dir.join(".gitignore"), "ignored.txt\n").unwrap();
		fs_ext::write(dir.join("main.rs"), "fn main() {}\n").unwrap();
		git(&dir, &["add", "."]);
		git(&dir, &["commit", "-q", "--no-verify", "-m", "initial"]);
		dir
	}

	/// The diff hash of the tree at `dir`, failing on a clean one.
	async fn diff_hash(dir: &AbsPath) -> SmolStr {
		match DeploySource::read(dir).await {
			DeploySource::Git {
				diff_hash: Some(diff_hash),
				..
			} => diff_hash,
			other => panic!("expected a dirty checkout, got {other:?}"),
		}
	}

	/// A clean checkout names its commit and carries no diff, from a
	/// subdirectory and on a detached head alike.
	#[beet_core::test]
	async fn records_a_clean_checkout() {
		let dir = committed_repo();
		let expected = DeploySource::Git {
			commit: git(&dir, &["rev-parse", "HEAD"]).into(),
			describe: git(&dir, &["describe", "--always", "--dirty"]).into(),
			diff_hash: None,
		};
		DeploySource::read(&dir).await.xpect_eq(expected.clone());

		git(&dir, &["checkout", "-q", "--detach"]);
		fs_ext::create_dir_all(dir.join("crates")).unwrap();
		DeploySource::read(&dir.join("crates"))
			.await
			.xpect_eq(expected.clone());
		// an ignored file is not a change
		fs_ext::write(dir.join("ignored.txt"), "scratch").unwrap();
		DeploySource::read(&dir).await.xpect_eq(expected);
	}

	/// A dirty tree hashes its uncommitted changes, tracked and untracked
	/// alike, and the same tree always hashes the same.
	#[beet_core::test]
	async fn hashes_a_dirty_tree_stably() {
		let dir = committed_repo();
		fs_ext::write(dir.join("main.rs"), "fn main() { todo!() }\n").unwrap();
		let DeploySource::Git { describe, .. } = DeploySource::read(&dir).await
		else {
			panic!("expected a git source");
		};
		describe.as_str().xpect_ends_with("-dirty");
		let tracked = diff_hash(&dir).await;
		diff_hash(&dir).await.xpect_eq(tracked.clone());
		// staging the same change is the same tree
		git(&dir, &["add", "main.rs"]);
		diff_hash(&dir).await.xpect_eq(tracked.clone());

		// an untracked file is a change, and so is its content
		fs_ext::write(dir.join("notes.md"), "draft").unwrap();
		let untracked = diff_hash(&dir).await;
		untracked.xpect_not_eq(tracked.clone());
		diff_hash(&dir).await.xpect_eq(untracked.clone());
		fs_ext::write(dir.join("notes.md"), "second draft").unwrap();
		diff_hash(&dir).await.xpect_not_eq(untracked);
		// an ignored one is not
		fs_ext::remove(dir.join("notes.md")).unwrap();
		fs_ext::write(dir.join("ignored.txt"), "scratch").unwrap();
		diff_hash(&dir).await.xpect_eq(tracked);
	}

	/// Outside a checkout the source is unknown, with git's reason, and the
	/// read still succeeds.
	#[beet_core::test]
	async fn outside_a_repository_is_unknown() {
		let dir = TempDir::new().unwrap();
		let DeploySource::Unknown { reason } = DeploySource::read(&dir).await
		else {
			panic!("expected an unknown source");
		};
		reason.as_str().xpect_contains("not a git repository");

		// a repository with no commit has no source to name either
		git(&dir, &["init", "-q"]);
		DeploySource::read(&dir)
			.await
			.xpect_eq(DeploySource::unknown("the repository has no commit"));
	}
}
