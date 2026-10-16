//! A repo's deploy accounts, one per provider that has accounts to give, and
//! the verb that closes them.

use beet_core::prelude::*;
use beet_net::prelude::*;
use std::fmt;
use std::sync::Arc;

/// The erased handle over one provider's deploy account, landed on the entity
/// of its declaration (`<AwsAccount/>`) and collected by
/// [`DeployAccountQuery`].
///
/// A deploy account is the provider account a repo's stacks deploy into, the
/// repo's own rather than one shared with every other repo: what isolates one
/// repo's resources and credentials from another's outright, where a policy
/// scoped by name only narrows them. Like a
/// [`DeployCredential`](crate::prelude::DeployCredential) it is
/// infrastructure, declared at the package root and converged by an elevated
/// deploy (the provider's credential converges its account first, since the
/// credential lives in it), so no verb names a provider. A provider with no
/// accounts to give by api, ie Cloudflare outside its tenant programme, simply
/// declares none.
#[derive(Clone, Component)]
pub struct DeployAccount {
	provider: Arc<dyn DeployAccountProvider>,
}

impl fmt::Debug for DeployAccount {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("DeployAccount")
			.field("provider", &self.provider.id())
			.field("name", &self.provider.name())
			.finish()
	}
}

impl DeployAccount {
	/// The erased handle over `provider`.
	pub fn new(provider: impl DeployAccountProvider) -> Self {
		Self {
			provider: Arc::new(provider),
		}
	}

	/// See [`DeployAccountProvider::id`].
	pub fn id(&self) -> &'static str { self.provider.id() }

	/// See [`DeployAccountProvider::name`].
	pub fn name(&self) -> SmolStr { self.provider.name() }

	/// See [`DeployAccountProvider::close`].
	pub async fn close(&self, caller: &AsyncEntity) -> Result<String> {
		self.provider.close(caller.clone()).await
	}
}

/// One provider's deploy account, see [`DeployAccount`]. Implemented by the
/// declaration itself, so the settings it carries are the provider's.
pub trait DeployAccountProvider: 'static + Send + Sync {
	/// Stable provider discriminator, ie `aws`, matching its credential's.
	fn id(&self) -> &'static str;

	/// The account's name, which it is found by: the repo's.
	fn name(&self) -> SmolStr;

	/// Close the account, asking a person for the provider's human factor,
	/// refusing while any stack of the package still holds state, and saying
	/// what closing means.
	fn close(&self, caller: AsyncEntity) -> SendBoxedFuture<Result<String>>;
}

/// Every [`DeployAccount`] this launch declares.
#[derive(SystemParam)]
pub struct DeployAccountQuery<'w, 's> {
	accounts: Query<'w, 's, &'static DeployAccount>,
}

impl DeployAccountQuery<'_, '_> {
	/// Every declared account, refusing two of one provider: a repo deploys
	/// into one account per provider, and two would split its credential.
	pub fn all(&self) -> Result<Vec<DeployAccount>> {
		let mut seen = HashSet::<&'static str>::default();
		let mut all = Vec::new();
		for account in self.accounts.iter() {
			if !seen.insert(account.id()) {
				bevybail!(
					"this launch declares two `{}` deploy accounts, and a repo \
					deploys into one per provider: keep one declaration",
					account.id()
				);
			}
			all.push(account.clone());
		}
		all.sort_by_key(|account| account.id());
		all.xok()
	}
}

/// Observer: land the [`DeployAccount`] a declaration of `T` declares on its
/// entity, the declaration being the provider.
pub(crate) fn attach_deploy_account<
	T: 'static + Send + Sync + Clone + Component + DeployAccountProvider,
>(
	ev: On<Insert<T>>,
	declarations: Query<&T>,
	mut commands: Commands,
) {
	if let Ok(declaration) = declarations.get(ev.entity) {
		commands
			.entity(ev.entity)
			.insert(DeployAccount::new(declaration.clone()));
	}
}

/// `close-account --elevated`: close every deploy account the package
/// declares. A package verb rather than a side effect of any stack's
/// `destroy`, since closing is the package's act and the most final one there
/// is; each provider refuses while a stack of the package still holds state.
///
/// ```bsx
/// <CloseDeployAccounts/>
/// ```
#[action(route = "close-account")]
#[derive(Default, Component, Reflect)]
#[reflect(Component, Default)]
pub async fn CloseDeployAccounts(
	/// Close them: the account and everything left in it go, after the
	/// provider's own grace period. Required, since nothing about closing is a
	/// plain deploy's to do.
	#[param]
	elevated: bool,
	cx: ActionContext<Request>,
) -> Result<Response> {
	let accounts = cx
		.caller
		.with_state::<DeployAccountQuery, _>(|_, query| query.all())
		.await??;
	if accounts.is_empty() {
		bevybail!(
			"this package declares no deploy account, so there is nothing to \
			close"
		);
	}
	if !elevated {
		bevybail!(
			"closing {} is elevated, and final: run `close-account \
			--elevated`, which asks for each provider's human factor",
			accounts
				.iter()
				.map(|account| format!(
					"the {} account `{}`",
					account.id(),
					account.name()
				))
				.collect::<Vec<_>>()
				.join(" and ")
		);
	}
	let mut out = String::new();
	for account in accounts {
		out.push_str(&format!(
			"{}: {}\n",
			account.id(),
			account.close(&cx.caller).await?
		));
	}
	Response::ok_text(out).xok()
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;
	use beet_router::prelude::*;

	/// An account that records being closed.
	#[derive(Clone)]
	struct Closed(Store<bool>);

	impl DeployAccountProvider for Closed {
		fn id(&self) -> &'static str { "closed" }

		fn name(&self) -> SmolStr { "beetmash".into() }

		fn close(
			&self,
			_caller: AsyncEntity,
		) -> SendBoxedFuture<Result<String>> {
			let closed = self.0;
			Box::pin(async move {
				closed.set(true);
				"closed".to_string().xok()
			})
		}
	}

	/// Closing is never a plain verb: without `--elevated` it refuses naming
	/// every account, and with it each provider closes its own.
	#[beet_core::test]
	async fn closes_only_when_elevated() {
		let closed = Store::new(false);
		let mut world = (AsyncPlugin, RouterPlugin, InfraPlugin).into_world();
		world.spawn(DeployAccount::new(Closed(closed)));
		let root = world
			.spawn((Router::with_defaults(), children![CloseDeployAccounts]))
			.flush();
		let refused = world
			.entity_mut(root)
			.exchange(Request::get("close-account"))
			.await;
		refused.status().is_ok().xpect_false();
		refused
			.text()
			.await
			.unwrap()
			.xpect_contains("the closed account `beetmash`")
			.xpect_contains("close-account --elevated");
		closed.get().xpect_false();
		world
			.entity_mut(root)
			.exchange(Request::from_cli_str("close-account --elevated"))
			.await
			.status()
			.xpect_eq(StatusCode::OK);
		closed.get().xpect_true();
	}
}
