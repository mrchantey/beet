//! `<AtprotoAccount/>`: an account declared once and named from anywhere.
#[cfg(feature = "atproto")]
use crate::prelude::*;
use beet_core::prelude::*;

/// An account whose repo this document reads or writes:
///
/// ```html
/// <AtprotoAccount bx:ref="beet_org" handle="beet.org"
/// 	did="did:plc:vgxy56shhs3t3b2mu2avizjf" display_name="Beet"
/// 	secret="BEET_ORG_APP_PASSWORD"/>
/// ```
///
/// Plain data registered in every build, so a document declaring accounts
/// loads whole anywhere: the served site and the deploy verbs both read them.
/// Under the `atproto` feature its attach lands the account's [`Pds`] on the
/// entity, an `XrpcPds` whose PDS is resolved from the did on first use, so
/// declaring an account costs nothing until something reads its repo.
///
/// The did is the identity and the handle the name it is known by; a handle
/// can move, so every address written names the did. `secret` names the env
/// var holding an app password (a `secrets.toml` record with `role =
/// "env_var"`); without one the repo reads and refuses every write. A did has
/// no default, so neither has an account: a tag names at least its handle and
/// did.
#[derive(Debug, Clone, PartialEq, Eq, Get, SetWith, Component, Reflect)]
#[reflect(Component)]
pub struct AtprotoAccount {
	/// The name the account is known by, ie `pete.beet.org`.
	handle: SmolStr,
	/// The account's permanent identity.
	did: Did,
	/// The name a person reads, matched against a page's author.
	#[reflect(default)]
	display_name: SmolStr,
	/// The env var holding the account's app password.
	#[set_with(unwrap_option, into)]
	#[reflect(default)]
	secret: Option<SmolStr>,
	/// Repost every announcement this account contributed to.
	#[reflect(default)]
	auto_repost: bool,
}

impl AtprotoAccount {
	/// The account `handle` known by `did`, read only until it names a
	/// secret.
	pub fn new(
		handle: impl Into<SmolStr>,
		did: Did,
		display_name: impl Into<SmolStr>,
	) -> Self {
		Self {
			handle: handle.into(),
			did,
			display_name: display_name.into(),
			secret: None,
			auto_repost: false,
		}
	}
}

/// The declared accounts, by what other declarations know them by.
#[derive(SystemParam)]
pub struct AtprotoAccountQuery<'w, 's> {
	accounts: Query<'w, 's, (Entity, &'static AtprotoAccount)>,
}

impl AtprotoAccountQuery<'_, '_> {
	/// The one account whose `display_name` is `name`, ie a page's author as
	/// a document contributor. None or several is an error listing the
	/// declared names, since a guess would credit the wrong person.
	pub fn by_display_name(
		&self,
		name: &str,
	) -> Result<(Entity, &AtprotoAccount)> {
		let mut matches = self
			.accounts
			.iter()
			.filter(|(_, account)| account.display_name() == name);
		match (matches.next(), matches.next()) {
			(Some(found), None) => found.xok(),
			(Some(_), Some(_)) => bevybail!(
				"several `<AtprotoAccount/>`s are named `{name}`: a display \
				 name must pick out one account"
			),
			(None, _) => bevybail!(
				"no `<AtprotoAccount/>` is named `{name}`, the declared names \
				 are: {}",
				self.accounts
					.iter()
					.map(|(_, account)| account.display_name().as_str())
					.collect::<Vec<_>>()
					.join(", ")
			),
		}
	}
}

/// Observer: land the [`Pds`] an `<AtprotoAccount/>` declares on its entity.
#[cfg(feature = "atproto")]
pub(super) fn attach_account(
	ev: On<Insert<AtprotoAccount>>,
	accounts: Query<&AtprotoAccount>,
	mut commands: Commands,
) -> Result {
	let account = accounts.get(ev.entity)?;
	let auth = account
		.secret()
		.as_ref()
		.map(|secret| AppPassword::new(account.did().clone(), secret.clone()));
	commands.entity(ev.entity).try_insert(Pds::new(
		XrpcPds::new(account.did().clone()).with_auth(auth),
	));
	Ok(())
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	fn account(name: &str) -> AtprotoAccount {
		AtprotoAccount::new(
			"a.example.com",
			Did::parse(EmulatorPds::TEST_DID).unwrap(),
			name,
		)
	}

	#[beet_core::test]
	fn finds_one_account_by_display_name() {
		let mut world = World::new();
		world.spawn(account("Pete"));
		world.spawn(account("Beet"));
		world.spawn(account("Beet"));
		world.with_state::<AtprotoAccountQuery, _>(|accounts| {
			accounts
				.by_display_name("Pete")
				.unwrap()
				.1
				.display_name()
				.xpect_eq("Pete");
			accounts
				.by_display_name("Beet")
				.unwrap_err()
				.to_string()
				.xpect_contains("several");
			accounts
				.by_display_name("Nobody")
				.unwrap_err()
				.to_string()
				.xpect_contains("Pete");
		});
	}
}
