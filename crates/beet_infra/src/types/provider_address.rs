//! Where a stack's resources land at each provider: the aws region, the
//! Cloudflare account and the Cloudflare zone, each a component resolved by
//! ancestry from the entity that asks.
//!
//! An address is declaration data, public in every dashboard url, so it lives
//! in committed markup as a spread on the stack or any ancestor
//! (`<Router {AwsRegion("ap-southeast-2")}>` for every stack under it, `<Stack
//! {(CloudflareAccount{id:".."}, CloudflareZone{domain:"beetmash.com",
//! id:".."})}>` for one) and a block that genuinely needs another zone or
//! region carries the same spread itself. [`StackQuery::resolve`] reads the
//! nearest ancestor-or-self of the asking entity into the [`ResolvedStack`],
//! whose accessors fail by name when a consumer needs one that is not
//! declared: the environment is never consulted, since an address that can
//! silently fall back is the class of mistake an implicit store resolution
//! was removed for, and here a wrong answer replaces every resource in the
//! stack.

use beet_core::prelude::*;

/// The aws region every resource beneath this entity deploys into, ie
/// `{AwsRegion("ap-southeast-2")}`.
///
/// One per stack in effect: the tofu aws provider is configured once per
/// stack config, so a block's own spread addresses its runtime store while
/// terraform creates the resource in the stack's, which is why no block
/// carries a region field. A second region is a nested `<Stack>`.
#[derive(Debug, Default, Clone, PartialEq, Eq, Component, Reflect)]
#[reflect(Component, Default)]
pub struct AwsRegion(pub SmolStr);

impl AwsRegion {
	pub fn new(region: impl Into<SmolStr>) -> Self { Self(region.into()) }
}

/// The Cloudflare account a stack's Cloudflare resources belong to (an R2
/// bucket, a load-balancer pool, a Worker), ie `{CloudflareAccount{id:".."}}`.
#[derive(Debug, Default, Clone, PartialEq, Eq, Component, Reflect)]
#[reflect(Component, Default)]
pub struct CloudflareAccount {
	/// The account id, as the dashboard url shows it.
	pub id: SmolStr,
}

impl CloudflareAccount {
	pub fn new(id: impl Into<SmolStr>) -> Self { Self { id: id.into() } }
}

/// The Cloudflare zone a stack publishes its records into, ie
/// `{CloudflareZone{domain:"beetmash.com", id:".."}}`. The `domain` is the
/// zone's apex, so a record whose name is not under it is refused at render
/// ([`ResolvedStack::cloudflare_zone`]), the cheap guard that catches a
/// wrong spread.
#[derive(Debug, Default, Clone, PartialEq, Eq, Component, Reflect)]
#[reflect(Component, Default)]
pub struct CloudflareZone {
	/// The zone's apex, ie `beetmash.com`.
	pub domain: SmolStr,
	/// The zone id, as the dashboard url shows it.
	pub id: SmolStr,
}

impl CloudflareZone {
	pub fn new(domain: impl Into<SmolStr>, id: impl Into<SmolStr>) -> Self {
		Self {
			domain: domain.into(),
			id: id.into(),
		}
	}

	/// Whether `name` is the zone's apex or a name under it.
	pub fn holds(&self, name: &str) -> bool {
		name == self.domain
			|| name
				.strip_suffix(self.domain.as_str())
				.is_some_and(|prefix| prefix.ends_with('.'))
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// The apex and every name under it are in the zone; a lookalike suffix
	/// and a parent are not.
	#[beet_core::test]
	fn a_zone_holds_its_names() {
		let zone = CloudflareZone::new("beetmash.com", "z");
		zone.holds("beetmash.com").xpect_true();
		zone.holds("mail.beetmash.com").xpect_true();
		zone.holds("_dmarc.news.beetmash.com").xpect_true();
		zone.holds("notbeetmash.com").xpect_false();
		zone.holds("com").xpect_false();
	}

	/// The three addresses author as spreads and resolve by ancestry: a
	/// region on the root serves every stack under it, a stack's own zone and
	/// account serve it, and an entity outside every stack still resolves the
	/// addresses above it.
	#[beet_core::test]
	fn addresses_resolve_by_ancestry() {
		let mut world = InfraPlugin.into_world();
		world.init_resource::<PackageConfig>();
		let root = BsxTemplate::parse_entry(
			&world,
			r#"<Router {AwsRegion("ap-southeast-2")}>
				<Stack app_name="mail" {(CloudflareAccount{id:"acct"}, CloudflareZone{domain:"beetmash.com", id:"zone"})}>
					<div/>
				</Stack>
				<Stack app_name="other" {AwsRegion("us-west-2")}/>
			</Router>"#,
		)
		.unwrap()
		.spawn(&mut world)
		.unwrap();
		world.flush();
		let stacks = world
			.query_filtered::<Entity, With<Stack>>()
			.iter(&world)
			.collect::<Vec<_>>();
		let leaf = world.entity(stacks[0]).get::<Children>().unwrap()[0];
		world.with_state::<StackQuery, _>(|query| {
			let mail = query.resolve(leaf);
			mail.region().unwrap().as_str().xpect_eq("ap-southeast-2");
			mail.cloudflare_account()
				.unwrap()
				.id
				.as_str()
				.xpect_eq("acct");
			mail.cloudflare_zone_holding("mail.beetmash.com")
				.unwrap()
				.id
				.as_str()
				.xpect_eq("zone");
			mail.cloudflare_zone_holding("mail.example.org")
				.unwrap_err()
				.to_string()
				.xpect_contains("beetmash.com")
				.xpect_contains("mail.example.org");
			let other = query.resolve(stacks[1]);
			other.region().unwrap().as_str().xpect_eq("us-west-2");
			other
				.cloudflare_account()
				.unwrap_err()
				.to_string()
				.xpect_contains("CloudflareAccount");
			query
				.resolve(root)
				.region()
				.unwrap()
				.as_str()
				.xpect_eq("ap-southeast-2");
		});
	}
}
