//! The deploy-time half of a served entry document.

use crate::prelude::*;
use beet_core::prelude::*;

/// Declared on the compute a deploy ships: this process serves the stack's repo
/// store, so it reads the document THIS deploy published and no other.
///
/// It exists because that is an agreement between two declarations and neither
/// states it alone. The compute is given `<store>/<deploy id>`, and that prefix
/// only holds a document because the store declared
/// [`deploy_versioned`](S3BucketBlock::deploy_versioned). Turn the flag off and
/// the deploy still succeeds: it publishes to the store root, the process reads
/// a prefix nothing wrote, and every request 404s with no error anywhere.
///
/// So a compute that serves the repo store says so, and [`assert_repo_buckets`]
/// holds the pair together at render time, before any tofu invocation.
///
/// Where the prefix reaches the process is the target's business and not this
/// type's. A lambda bakes it into the zip's `bootstrap` script, a container into
/// its `CMD`, and a machine whose boot config cannot change per deploy resolves
/// it from the release pointer at every start ([`ArtifactLedger::repo`]). All
/// three read the same prefix, which is why there is one marker rather than one
/// per channel.
#[derive(Debug, Default, Clone, Copy, PartialEq, Component, Reflect)]
#[reflect(Component, Default)]
pub struct RepoBucket;

impl RepoBucket {
	/// The label a stack declares its repo store under, and the one string both
	/// ends of the agreement share. `RepoBucket::LABEL` re-exports
	/// this rather than restating it, so the builders that hand the prefix to a
	/// process and the assertion that checks it cannot drift.
	pub const LABEL: &'static str = "repo";
}

/// Render-set: fail a stack that serves its entry document from a repo store it
/// does not declare, or declares unversioned. See [`RepoBucket`].
pub(crate) fn assert_repo_buckets(
	mut scopes: AncestorQuery<&mut RenderScope>,
	stacks: StackQuery,
	repos: Query<Entity, With<RepoBucket>>,
	stores: Query<&S3BucketBlock>,
) {
	for entity in repos.iter() {
		let Ok(declared) = stacks.declared(entity) else {
			continue;
		};
		let store = declared
			.iter()
			.filter_map(|entity| stores.get(*entity).ok())
			.find(|store| store.label() == RepoBucket::LABEL);
		let err = match store {
			Some(store) if store.deploy_versioned() => continue,
			Some(store) => bevyhow!(
				"the store '{}' this deploy serves its entry document from \
				 declares `deploy_versioned=false`, but the process it ships \
				 reads `<store>/<deploy id>`. The deploy would publish to the \
				 store root and the process would read a prefix nothing wrote: \
				 delete the attribute, since it defaults to true.",
				store.label()
			),
			None => bevyhow!(
				"this deploy serves its entry document from a store labelled \
				 '{}', which nothing under this stack declares. Add \
				 `<S3BucketBlock label=\"{}\"/>`.",
				RepoBucket::LABEL,
				RepoBucket::LABEL
			),
		};
		if let Ok(mut scope) = scopes.get_mut(entity) {
			scope.error(err);
		}
	}
}

#[cfg(test)]
mod tests {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// Both halves of the agreement, checked at render time, ie before any tofu
	/// invocation: the deployed shape of either mistake is a successful deploy
	/// that serves nothing.
	#[beet_core::test]
	fn a_served_repo_store_must_be_deploy_versioned() {
		RenderScope::test_render(|parent| {
			parent.spawn(
				S3BucketBlock::new(RepoBucket::LABEL)
					.with_deploy_versioned(false),
			);
			parent.spawn(RepoBucket);
		})
		.0
		.finish()
		.unwrap_err()
		.to_string()
		.xpect_contains("deploy_versioned=false");
		// ..and a stack that declares no repo store at all
		RenderScope::test_render(|parent| {
			parent.spawn(RepoBucket);
		})
		.0
		.finish()
		.unwrap_err()
		.to_string()
		.xpect_contains("nothing under this stack declares");
		// the default is what a served store wants, so the pair is silent
		RenderScope::test_render(|parent| {
			parent.spawn(S3BucketBlock::new(RepoBucket::LABEL));
			parent.spawn(RepoBucket);
		})
		.0
		.finish()
		.unwrap();
		// ..and a data store beside it is none of this assertion's business
		RenderScope::test_render(|parent| {
			parent.spawn(S3BucketBlock::new(RepoBucket::LABEL));
			parent.spawn(
				S3BucketBlock::new("mail-blobs").with_deploy_versioned(false),
			);
			parent.spawn(RepoBucket);
		})
		.0
		.finish()
		.unwrap();
	}
}
