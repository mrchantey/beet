//! The `secrets` verbs, one file per verb, each a route action carrying its
//! own params type so `--help` documents its flags. Every verb acts on a
//! secrets document named by `--document=<label or path>`, the declared one
//! by default ([`DocumentParams`]); every verb but `ls` resolves its
//! identity through `AgeIdentityFile::require`, failing with the `keygen`
//! guidance when none resolves. Values never reach a log: `get` prints its
//! value deliberately as its response, everything else answers with names,
//! counts and paths. Age files by path are the `vault` verbs' business.

mod check;
#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
mod exec;
mod get;
mod ls;
mod rekey;
mod rm;
mod secrets_routes;
mod set;

pub use check::*;
#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
pub use exec::*;
pub use get::*;
pub use ls::*;
pub use rekey::*;
pub use rm::*;
pub use secrets_routes::*;
pub use set::*;

use crate::prelude::*;
use beet_core::prelude::*;

/// The `--document` every document verb takes.
#[derive(Reflect)]
pub(crate) struct DocumentParams {
	/// The document: a declared `<Secrets>` label, or the path or store uri
	/// of an undeclared file (`~/personal.toml`,
	/// `s3://bucket/secrets/x.toml`). Defaults to the one declared document,
	/// or `secrets.toml` beside the entry when none is.
	pub document: Option<String>,
}

impl DocumentParams {
	/// The document the request names, resolved from `caller`.
	pub(crate) async fn resolve(
		request: &Request,
		caller: &AsyncEntity,
	) -> Result<SecretsHandle> {
		let params = request.parse_params::<Self>()?;
		SecretsHandle::resolve(caller, params.document.as_deref()).await
	}
}

/// The `:name` segment of `get/:name`, `set/:name` and `rm/:name`.
pub(crate) fn name_param(request: &Request) -> Result<SmolStr> {
	request
		.get_param("name")
		.map(str::trim)
		.filter(|name| !name.is_empty())
		.map(SmolStr::new)
		.ok_or_else(|| {
			bevyhow!(
				"a record name is required, ie `secrets/{} OPENAI_API_KEY`",
				request.path().last().map(SmolStr::as_str).unwrap_or("get")
			)
		})
}
