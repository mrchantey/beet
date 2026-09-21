//! The secrets document: a plaintext index of records plus one age blob per
//! group, sealed and opened with the vault layer (`crate::vault`).
//!
//! - [`SecretsDocument`]: a [`SecretsGroup`] per recipient list, each the
//!   index of its [`SecretRecord`]s, and one sealed blob per group, opened
//!   into [`OpenSecrets`]
//! - [`SecretRotation`]: how a record's secret is rotated, declared at mint; plain
//!   data in every build, since a stack declaration renders it
//! - [`Secrets`]: the `<Secrets path=".."/>` declaration, and the runner's
//!   convention for the same file
//!
//! ## Cheatsheet
//!
//! The rules every consumer of this subsystem relies on; the tutorial at
//! `site/routes/docs/secrets.md` walks them, `beet_net::secrets` holds the
//! store I/O and the verbs, and `beet_infra`'s README "Secrets" the stack
//! side (`SecretStore`, exports, restore, revoke).
//!
//! - **One identity per human, in the OS config, never passed in.**
//!   `~/.config/beet/age/keys.txt`, found by [`AgeIdentityFile::discover`](crate::vault::AgeIdentityFile::discover)
//!   (`BEET_AGE_IDENTITY` first, for an agent or a CI runner: a path or an
//!   inline key). A new machine restores a backup (`vault/restore-identity`),
//!   never runs `keygen` twice. Not a `BootstrapConfig` knob: a knob lands on
//!   argv, an identity never does.
//! - **The document is `secrets.toml` beside the entry, committed.**
//!   `[groups.<g>]` lists recipients and `[groups.<g>.secrets.NAME]` each
//!   record's metadata (`role`, `note`, `rotation`, `modified`, `address`),
//!   `[sealed]` holds one armored age file per group. No `.age` suffix: the
//!   file is plaintext, its blobs are the age files, and `age -d -i
//!   ~/.config/beet/age/keys.txt` on a pasted blob is the escape hatch.
//! - **The sealed side is the truth, the index its mirror.** Every record's
//!   metadata is sealed beside its value; `open` refuses an index that
//!   disagrees, so a hand edit never promotes a secret. Only the `recipients`
//!   lists are edited by hand; a note or rotation changes through
//!   `secrets/set NAME --from-env --note=.. --rotation=..`, which re-seals
//!   the value the launch loaded.
//! - **Everything is in a group; `default` is the one you get.** Humans sit
//!   in every group, an agent only in `agents`; a writer must be a member of
//!   the group it writes. A list edit takes effect on the next `set` of that
//!   group or on `secrets/rekey`; until then `check` reports the drift.
//! - **`EnvVar` is the only role.** The launch reads every top-level
//!   unconditional `<Secrets>` out of the entry prescan and sets its `EnvVar`
//!   records before the entry builds (process environment wins, then `.env`,
//!   then the document), so nothing reads a `.env` and no `main` knows. The
//!   test runner loads the nearest `secrets.<format>` above the cwd
//!   ([`Secrets::load_env_vars_nearest`]). A record with no role is kept and
//!   viewed, read by a system through [`OpenSecrets`].
//! - **No identity is one warning, never an error.** A cloud box and a
//!   contributor without the key build and run minus the records.
//! - **Values never reach a log.** `Debug` redacts an identity, a sealed
//!   record and an opened [`Secret`]; `secrets/get` and `vault/decrypt` print
//!   deliberately as their response. `set` takes its value from a no-echo
//!   prompt, a pipe, `--from-env` or `--generate` ([`Secret::generate`], the
//!   one mint every credential draws from), never argv by preference.
//! - **Every mint names its rotation.** [`SecretRotation`] is
//!   `replace:<resource>`, `remint` or `manual:<how>` (the url first, one
//!   step per line); a mint site cannot omit it, and `secrets/revoke` runs
//!   what it can and prints the rest.
//! - **Two nouns, two flags.** `secrets/*` verbs take
//!   `--document=<label or path>` (the declaration labelled `secrets`, else
//!   the sole one, else `secrets.toml` beside the entry); `vault/*` verbs
//!   take `--vault=<path or uri>` for one age file by path.

#[cfg(feature = "vault")]
mod open_secrets;
#[cfg(feature = "vault")]
mod secret_record;
mod secret_rotation;
#[cfg(feature = "vault")]
mod secrets;
#[cfg(feature = "vault")]
mod secrets_document;

#[cfg(feature = "vault")]
pub use open_secrets::*;
#[cfg(feature = "vault")]
pub use secret_record::*;
pub use secret_rotation::*;
#[cfg(feature = "vault")]
pub use secrets::*;
#[cfg(feature = "vault")]
pub use secrets_document::*;
