//! `<StandardSiteIndexWrite/>`: the derived index rewritten from the repo.
use super::publishing::*;
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// The last step of a publish sequence: rewrite the publication's
/// [`StandardSiteIndex`] from the repo by running the declaration's own
/// producer ([`Derivation::rewrite`]), whatever its freshness, since an index
/// names no sources that could go stale. An index matching the repo is left
/// byte for byte as it was, so a second run changes nothing in git. The
/// served site in this process reads the new index at once.
///
/// Under `--dry-run` it reports whether the index would change and writes
/// nothing; a dry run before the first publish finds no publication to index.
#[action]
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
pub async fn StandardSiteIndexWrite(
	publish: Params<StandardSiteWriteParams>,
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let world = cx.world();
	let publishing = Publishing::resolve(&cx.caller).await?;
	let derivation = publishing.declaration.derivation();
	let written = match publish.0.dry_run {
		true => {
			let Some(index) = publishing.records().await?.index() else {
				info!(
					"no publication at `{}` yet: the index waits for the first \
					 publish",
					publishing.url
				);
				return Ok(Pass(cx.input));
			};
			derivation
				.read(&publishing.store)
				.await?
				.map(|body| body.get_typed::<StandardSiteIndex>())
				.transpose()?
				.is_none_or(|current| current != index)
		}
		false => {
			let written = Derivation::rewrite(
				&world,
				publishing.entity,
				&publishing.store,
			)
			.await?;
			StandardSiteIndex::load(&world.entity(publishing.entity)).await?;
			written
		}
	};
	let verb = if written { "updated" } else { "unchanged" };
	info!("{verb:<9} {}", derivation.path);
	Ok(Pass(cx.input))
}
