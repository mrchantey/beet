//! `<StandardSiteIndexWrite/>`: the derived index rewritten from the public repo.
use super::publishing::*;
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// The last step of a publish sequence: rewrite the publication's
/// [`StandardSiteIndex`] from the public repo by running the declaration's own
/// producer ([`Derivation::rewrite`]), whatever its freshness, since an index
/// names no sources that could go stale. An index matching the public repo is left
/// byte for byte as it was, so a second run changes nothing in git. The
/// served site in this process reads the new index at once.
///
/// Under `--dry-run` it reports whether the index would change, comparing
/// the committed index with the one the run's dry repo projects, so the line
/// is the one the run itself would log; it writes nothing.
#[action]
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
pub async fn StandardSiteIndexWrite(
	publish: Params<StandardSiteWriteParams>,
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let ActionContext { caller, mut input } = cx;
	let world = caller.world().clone();
	let dry_run = publish.0.dry_run;
	let publishing = Publishing::resolve(&caller)
		.await?
		.in_run(&mut input, dry_run);
	let derivation = publishing.declaration.derivation();
	let written = match dry_run {
		true => {
			let Some(index) = publishing.records().await?.index() else {
				info!(
					"no publication at `{}` yet: the index waits for the first \
					 publish",
					publishing.url
				);
				return Ok(Pass(input));
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
	let verb = match written {
		true => ConvergeVerb::Updated,
		false => ConvergeVerb::Unchanged,
	};
	info!("{}", verb.line(dry_run, &derivation.path));
	Ok(Pass(input))
}
