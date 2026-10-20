//! Secrets: the store every secret a process reads or a deploy mints lives
//! behind, and the secrets document humans keep theirs in, a plaintext index
//! of records plus one age blob per group sealed with the vault layer
//! (`crate::vault`).
//!
//! In every build, since a deployed binary carries no vault:
//!
//! - [`SecretStore`]: the erased-provider seam, keyed by [`SecretRef`] and
//!   scoped by [`SecretScope`], resolved by [`SecretStoreQuery`]. Providers:
//!   [`EnvSecretStore`] (the launch's environment, read-only) here, AWS
//!   parameter store and a secrets document as a stack's store in `beet_infra`
//! - [`StoredSecret`]: one secret a process reads, a store and its key,
//!   landed by a declaration and named by a consumer's relation
//! - [`SecretRoll`]: how a secret is rolled, declared at mint, since a stack
//!   declaration renders it
//!
//! Behind `vault`:
//!
//! - [`SecretsDocument`]: a [`SecretsGroup`] per recipient list, each the
//!   index of its [`SecretRecord`]s, and one sealed blob per group, opened
//!   into [`OpenSecrets`]
//! - [`Secrets`]: the `<Secrets path=".."/>` declaration, and the runner's
//!   convention for the same file
//!
//! ## Cheatsheet
//!
//! The rules every consumer of this subsystem relies on; the tutorial at
//! `site/routes/docs/secrets.md` walks them, `beet_net::secrets` holds the
//! document I/O and the verbs, `beet_net`'s `AwsJsonClient` the signed
//! requests parameter store is reached by, and `beet_infra`'s README
//! "Secrets" the stack side (its store providers and declarations,
//! `<SecretBlock/>`, exports, restore, revoke).
//!
//! - **One identity per human, in the OS config, never passed in.**
//!   `~/.config/beet/age/keys.txt`, found by [`AgeIdentityFile::discover`](crate::vault::AgeIdentityFile::discover)
//!   (`BEET_AGE_IDENTITY` first, for an agent or a CI runner: a path or an
//!   inline key). A new machine restores a backup (`vault/restore-identity`),
//!   never runs `keygen` twice. Owner-only, or refused naming the `chmod`
//!   (`age -d -o` writes `0644`). Not a `BootstrapConfig` knob: a knob lands
//!   on argv, an identity never does.
//! - **The document is `secrets.toml` beside the entry, committed.**
//!   `[groups.<g>]` lists recipients and `[groups.<g>.secrets.NAME]` each
//!   record's metadata (`role`, `note`, `roll`, `modified`, `expires`,
//!   `address`),
//!   `[sealed]` holds one armored age file per group. No `.age` suffix: the
//!   file is plaintext, its blobs are the age files, and `age -d -i
//!   ~/.config/beet/age/keys.txt` on a pasted blob is the escape hatch.
//! - **The sealed side is the truth, the index its mirror.** Every record's
//!   metadata is sealed beside its value; `open` refuses an index that
//!   disagrees, so a hand edit never promotes a secret. Only the `recipients`
//!   lists are edited by hand; a note or roll changes through
//!   `secrets/set NAME --from-env --note=.. --roll=..`, which re-seals
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
//! - **One store, whichever side reads it.** Every secret a process reads
//!   or a deploy mints goes through a [`SecretStore`] by label; a consumer
//!   holds a [`StoredSecret`] (the store and the key) and never knows the
//!   provider. Absence is `None`, and an error only where the declaration
//!   says what supplies the value.
//! - **A deployed process never reads a record from its environment.** A
//!   secret a box needs is a `<SecretBlock/>` under its stack: the stack's
//!   tofu config owns it as a parameter whose value is write-only, which the
//!   apply fills from the record on the tofu child's environment, and the
//!   box reads it from the stack's secret store, while a local launch reads
//!   the same record from its environment ([`EnvSecretStore`]). So no value
//!   rides a rendered config (`user_data`, `main.tf.json`), a plan, the tofu
//!   state or a box's disk; the record's `modified` versions the parameter,
//!   so a roll rewrites it.
//! - **Values never reach a log.** `Debug` redacts an identity, a sealed
//!   record and an opened [`Secret`]; `secrets/get` and `vault/decrypt` print
//!   deliberately as their response. `set` takes its value from a no-echo
//!   prompt, a pipe, `--from-env` or `--generate` ([`Secret::generate`], the
//!   one mint every credential draws from), never argv by preference.
//! - **Every mint names its roll.** [`SecretRoll`] is
//!   `replace:<resource>`, `remint`, `elevated` (a deploy credential, which
//!   `<stack>/deploy --elevated --roll` replaces) or `manual:<how>` (the url
//!   first, one step per line); a mint site cannot omit it, and
//!   `secrets/revoke` runs what it can and prints the rest.
//! - **A credential with a lifetime says when it dies.** A mint that sets
//!   one records `expires`; the launch that sets the record into the
//!   environment warns inside its last
//!   [`SecretRecord::EXPIRY_NOTICE`], naming the roll, and `check`
//!   fails once it has passed. `set --expires=<date>` records a hand-made
//!   one's.
//! - **Two nouns, two flags.** `secrets/*` verbs take
//!   `--document=<label or path>` (the declaration labelled `secrets`, else
//!   the sole one, else `secrets.toml` beside the entry); `vault/*` verbs
//!   take `--vault=<path or uri>` for one age file by path.

mod env_secret_store;
#[cfg(feature = "vault")]
mod open_secrets;
#[cfg(feature = "vault")]
mod secret_record;
mod secret_ref;
mod secret_roll;
mod secret_store;
#[cfg(feature = "vault")]
mod secrets;
#[cfg(feature = "vault")]
mod secrets_document;
mod stored_secret;

pub use env_secret_store::*;
#[cfg(feature = "vault")]
pub use open_secrets::*;
#[cfg(feature = "vault")]
pub use secret_record::*;
pub use secret_ref::*;
pub use secret_roll::*;
pub use secret_store::*;
#[cfg(feature = "vault")]
pub use secrets::*;
#[cfg(feature = "vault")]
pub use secrets_document::*;
pub use stored_secret::*;
