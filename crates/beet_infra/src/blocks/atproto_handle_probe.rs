//! The assertion that a declared handle is a handle the network agrees with.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// `<AtprotoHandleProbe/>`: resolve every handle this stack publishes through
/// the AppView, and assert each answers with its account's did.
///
/// The one check that sees what an apply cannot. Terraform reports a converged
/// TXT record; whether that record makes `pete.beet.org` a handle depends on
/// the value being the `did=` form, on the name nesting under `_atproto.`, and
/// on no other record answering for the same name. Every one of those fails
/// silently, with a green apply, and shows up as an account whose handle is
/// marked invalid in the app.
///
/// Read-only and unauthenticated: it asks Bluesky's own resolver exactly what a
/// client asks, which is the only answer worth asserting. Resolves each handle
/// in turn, failing on the first that does not answer with its account's did.
#[action]
#[derive(Debug, Component, Reflect)]
#[reflect(Component, Default)]
pub async fn AtprotoHandleProbe(
	/// The AppView the handles are resolved through, ie a self-hosted instance.
	/// Defaults to the public one, which is the resolver the app itself uses.
	#[field(default = SmolStr::new_static(HandleResolver::PUBLIC_APPVIEW))]
	appview: SmolStr,
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let declared = cx
		.caller
		.with_world(AtprotoHandleQuery::published)
		.await??;
	if declared.is_empty() {
		bevybail!(
			"no atproto handle is published by this stage, so the probe would \
			 assert nothing: declare an <AtprotoHandleBlock/> naming at least \
			 one account, or run the probe from the stage its `dns_stage` names"
		);
	}

	let resolver = HandleResolver::service(appview);
	for (handle, did) in declared.iter() {
		let resolved = resolver.resolve(handle).await.map_err(|err| {
			bevyhow!(
				"'{handle}' does not resolve: the `_atproto.{handle}` record is \
				 missing, the zone has not propagated it yet, or the AppView is \
				 serving a stale failed resolution from before a fix, which it \
				 keeps for up to an hour: retry later. {err}"
			)
		})?;
		if resolved != *did {
			bevybail!(
				"'{handle}' resolves to {resolved}, not the declared {did}: \
				 another record is answering for this name"
			);
		}
		info!("{handle} -> {did}");
	}
	info!(
		"atproto handle probe passed: {} handle(s) resolve to their declared dids",
		declared.len()
	);
	Ok(Pass(cx.input))
}

/// The deploy tree a handle probe reads: the stack traversal, the blocks the
/// handles are declared on and the accounts they publish.
#[derive(SystemParam)]
pub struct AtprotoHandleQuery<'w, 's> {
	stacks: StackQuery<'w, 's>,
	blocks: Query<'w, 's, &'static AtprotoHandleBlock>,
	accounts: Query<'w, 's, &'static AtprotoAccount>,
}

impl AtprotoHandleQuery<'_, '_> {
	/// Every `(handle, did)` `entity`'s stack publishes from THIS stage,
	/// shaped to pass directly to [`AsyncEntity::with_world`].
	pub fn published(
		world: &mut World,
		entity: Entity,
	) -> Result<Vec<(String, Did)>> {
		world.with_state::<AtprotoHandleQuery, _>(|query| query.resolve(entity))
	}

	/// Every handle declared under `entity`'s stack that this stage publishes.
	///
	/// A block guarded by another stage's `dns_stage` emitted no record at all,
	/// so it is filtered out rather than asserted: probing it would fail a
	/// deploy that behaved exactly as declared.
	fn resolve(&self, entity: Entity) -> Result<Vec<(String, Did)>> {
		let (_, stack) = self.stacks.root(entity)?;
		let mut published = Vec::new();
		for block in self
			.stacks
			.declared(entity)?
			.iter()
			.filter_map(|child| self.blocks.get(*child).ok())
			.filter(|block| block.owns_names(&stack))
		{
			for handle in block.handles(&self.accounts)? {
				published.push((
					handle.handle(block.domain()),
					handle.did().clone(),
				));
			}
		}
		published.xok()
	}
}
