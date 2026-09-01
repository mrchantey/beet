use crate::bindings::*;
use crate::prelude::*;
use crate::terra::ResourceDef;
use beet_core::prelude::*;

/// A small private network: one vpc, a public and a private subnet in each of
/// two availability zones, and an internet gateway the public side routes
/// through.
///
/// Deliberately no NAT gateway. The private subnets stay on the vpc's main
/// route table, whose only route is the local cidr, so a private instance has
/// no path off the vpc at all. Giving it one costs about $32 a month, and the
/// resource this network exists for (a database) has nothing to reach out to.
/// A private workload that genuinely needs egress wants a vpc endpoint for the
/// one service it calls, not a gateway to the whole internet.
///
/// Authored directly from markup, ie `<VpcBlock bx:ref="net" label="net"/>`. A
/// consumer names the declaration entity through a [`VpcRef`] relation
/// (`{VpcRef($net)}`), and its render system reads this block off the target to
/// compose the terraform addresses, so both sides of every cross-block
/// reference go through the one block that emits the resources.
#[derive(
	Debug, Clone, Get, SetWith, Serialize, Deserialize, Component, Reflect,
)]
#[reflect(Component, Default)]
#[component(immutable, on_insert = ErasedBlock::on_insert::<Self>)]
pub struct VpcBlock {
	label: SmolStr,
	/// The network this vpc owns, which must be a `/16`: every subnet is a
	/// `/24` carved out of its third octet.
	cidr: SmolStr,
}

impl Default for VpcBlock {
	fn default() -> Self { Self::new("") }
}

impl VpcBlock {
	/// The default network. Private space, and a `/16` so the third octet is
	/// free for subnets to number themselves with.
	pub const CIDR: &'static str = "10.0.0.0/16";

	/// The label suffixes of the resources this block emits, which are also
	/// the `Name` tags they carry.
	pub const VPC: &'static str = "vpc";
	pub const GATEWAY: &'static str = "gateway";
	pub const PUBLIC_ROUTES: &'static str = "public-routes";
	pub const DEFAULT_ROUTE: &'static str = "default-route";

	pub fn new(label: impl Into<SmolStr>) -> Self {
		Self {
			label: label.into(),
			cidr: Self::CIDR.into(),
		}
	}

	/// This vpc's label with a resource suffix, ie `net--private-a`.
	pub fn suffix(&self, kind: &str) -> String {
		format!("{}--{kind}", self.label)
	}

	/// The terraform ident of one of this vpc's resources: what the block emits
	/// under, and what a consumer's interpolation resolves against, so a renamed
	/// resource is a compile-time move rather than a dangling interpolation
	/// discovered at apply.
	pub fn ident(&self, stack: &ResolvedStack, kind: &str) -> terra::Ident {
		stack.resource_ident(self.suffix(kind))
	}

	/// An interpolated reference to `field` of one of this vpc's resources.
	fn field_ref(
		&self,
		stack: &ResolvedStack,
		resource_type: &str,
		kind: &str,
		field: &str,
	) -> String {
		format!(
			"${{{resource_type}.{}.{field}}}",
			self.ident(stack, kind).label()
		)
	}

	/// The vpc id, ie what a security group or a subnet is created in.
	pub fn id(&self, stack: &ResolvedStack) -> String {
		self.field_ref(stack, "aws_vpc", Self::VPC, "id")
	}

	/// One subnet's id.
	pub fn subnet_id(
		&self,
		stack: &ResolvedStack,
		tier: SubnetTier,
		zone: &str,
	) -> String {
		self.field_ref(stack, "aws_subnet", &tier.kind(zone), "id")
	}

	/// Every subnet id of one tier, in availability-zone order. What a db
	/// subnet group or a load balancer is spread across.
	pub fn subnet_ids(
		&self,
		stack: &ResolvedStack,
		tier: SubnetTier,
	) -> Vec<SmolStr> {
		SubnetTier::AZ_SUFFIXES
			.iter()
			.map(|zone| self.subnet_id(stack, tier, zone).into())
			.collect()
	}

	/// The first two octets of [`cidr`](Self::cidr), ie the `10.0` every subnet
	/// numbers itself under. Errors on anything that is not a `/16`, since a
	/// narrower prefix has no third octet to give away and a wider one is not a
	/// vpc AWS will create.
	pub fn network_prefix(&self) -> Result<String> {
		let Some((address, prefix)) = self.cidr.split_once('/') else {
			bevybail!("vpc cidr '{}' has no prefix length", self.cidr);
		};
		let octets = address.split('.').collect::<Vec<_>>();
		if prefix != "16" || octets.len() != 4 {
			bevybail!(
				"vpc cidr '{}' must be a /16, ie '10.0.0.0/16': every subnet is a /24 carved out of its third octet",
				self.cidr
			);
		}
		for octet in &octets {
			octet.parse::<u8>().map_err(|_| {
				bevyhow!("vpc cidr '{}' is not an ipv4 address", self.cidr)
			})?;
		}
		if octets[2] != "0" || octets[3] != "0" {
			bevybail!(
				"vpc cidr '{}' is not the base address of its /16, ie '{}.{}.0.0/16'",
				self.cidr,
				octets[0],
				octets[1]
			);
		}
		format!("{}.{}", octets[0], octets[1]).xok()
	}

	/// The `/24` a subnet of `tier` in the `index`th availability zone takes.
	pub fn subnet_cidr(
		&self,
		tier: SubnetTier,
		index: usize,
	) -> Result<String> {
		format!("{}.{}.0/24", self.network_prefix()?, tier.octet() + index)
			.xok()
	}

	/// The `Name`/`Project`/`Stage` tags every resource here carries, so the
	/// console reads as the stack does.
	fn tags(
		&self,
		stack: &ResolvedStack,
		kind: &str,
	) -> std::collections::BTreeMap<SmolStr, SmolStr> {
		[
			(SmolStr::from("Name"), self.suffix(kind).as_str().into()),
			(SmolStr::from("Project"), stack.app_name().clone()),
			(SmolStr::from("Stage"), stack.stage().clone()),
		]
		.into_iter()
		.collect()
	}
}

impl Block for VpcBlock {
	fn label(&self) -> &SmolStr { &self.label }
}

impl EmitBlock for VpcBlock {
	fn emit(
		&self,
		stack: &ResolvedStack,
		_deployment: &Deployment,
		config: &mut terra::Config,
	) -> Result {
		self.emit(stack, config)
	}
}

impl VpcBlock {
	/// Emit the vpc, its subnets and the public side's routes.
	fn emit(
		&self,
		stack: &ResolvedStack,
		config: &mut terra::Config,
	) -> Result {
		let vpc = ResourceDef::new_secondary(
			self.ident(stack, Self::VPC),
			AwsVpcDetails {
				cidr_block: Some(self.cidr.clone()),
				// both on, so an instance resolves the private dns name of
				// anything else in the vpc (which is how it reaches the db).
				enable_dns_hostnames: Some(true),
				enable_dns_support: Some(true),
				tags: Some(self.tags(stack, Self::VPC)),
				..default()
			},
		);
		config.add_resource(&vpc)?;
		self.emit_subnets(stack, config, &vpc)?;
		self.emit_public_routes(stack, config, &vpc)?;
		Ok(())
	}
}

impl VpcBlock {
	/// One subnet per tier per availability zone. Two zones because a db subnet
	/// group needs at least two, and every AWS region has an `a` and a `b`.
	fn emit_subnets(
		&self,
		stack: &ResolvedStack,
		config: &mut terra::Config,
		vpc: &ResourceDef<AwsVpcDetails>,
	) -> Result {
		for tier in SubnetTier::ALL {
			for (index, zone) in SubnetTier::AZ_SUFFIXES.iter().enumerate() {
				let kind = tier.kind(zone);
				config.add_resource(&ResourceDef::new_secondary(
					self.ident(stack, &kind),
					AwsSubnetDetails {
						vpc_id: vpc.field_ref("id").into(),
						cidr_block: Some(
							self.subnet_cidr(*tier, index)?.into(),
						),
						availability_zone: Some(
							format!("{}{zone}", stack.region()).into(),
						),
						// a public subnet's instance gets a public address at
						// launch; a private one must never.
						map_public_ip_on_launch: Some(tier.is_public()),
						tags: Some(self.tags(stack, &kind)),
						..default()
					},
				))?;
			}
		}
		Ok(())
	}

	/// The route table the PUBLIC subnets share, with its default route to the
	/// internet gateway. There is no private table: leaving those subnets on
	/// the vpc's main table is what makes their lack of egress the default
	/// rather than a rule somebody has to remember to keep.
	fn emit_public_routes(
		&self,
		stack: &ResolvedStack,
		config: &mut terra::Config,
		vpc: &ResourceDef<AwsVpcDetails>,
	) -> Result {
		let gateway = ResourceDef::new_secondary(
			self.ident(stack, Self::GATEWAY),
			AwsInternetGatewayDetails {
				vpc_id: Some(vpc.field_ref("id").into()),
				tags: Some(self.tags(stack, Self::GATEWAY)),
				..default()
			},
		);
		let table = ResourceDef::new_secondary(
			self.ident(stack, Self::PUBLIC_ROUTES),
			AwsRouteTableDetails {
				vpc_id: vpc.field_ref("id").into(),
				tags: Some(self.tags(stack, Self::PUBLIC_ROUTES)),
				..default()
			},
		);
		let default_route = ResourceDef::new_secondary(
			self.ident(stack, Self::DEFAULT_ROUTE),
			AwsRouteDetails {
				route_table_id: table.field_ref("id").into(),
				destination_cidr_block: Some("0.0.0.0/0".into()),
				gateway_id: Some(gateway.field_ref("id").into()),
				..default()
			},
		);
		config
			.add_resource(&gateway)?
			.add_resource(&table)?
			.add_resource(&default_route)?;
		for zone in SubnetTier::AZ_SUFFIXES {
			let subnet = SubnetTier::Public.kind(zone);
			config.add_resource(&ResourceDef::new_secondary(
				self.ident(stack, &format!("{subnet}-routes")),
				AwsRouteTableAssociationDetails {
					subnet_id: Some(
						self.subnet_id(stack, SubnetTier::Public, zone).into(),
					),
					route_table_id: table.field_ref("id").into(),
					..default()
				},
			))?;
		}
		Ok(())
	}
}

/// Which side of a [`VpcBlock`] a subnet sits on: reachable from the internet,
/// or reachable only from inside the vpc.
#[derive(
	Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect,
)]
pub enum SubnetTier {
	/// Routed to the internet gateway, and instances launched here take a
	/// public address.
	Public,
	/// No route off the vpc at all, see [`VpcBlock`].
	#[default]
	Private,
}

impl SubnetTier {
	/// The availability zones a vpc spans, as region suffixes. Two, because a
	/// db subnet group requires at least two and every region has these.
	pub const AZ_SUFFIXES: &'static [&'static str] = &["a", "b"];
	pub const ALL: &'static [Self] = &[Self::Public, Self::Private];

	pub fn is_public(&self) -> bool { matches!(self, Self::Public) }

	pub fn label(&self) -> &'static str {
		match self {
			Self::Public => "public",
			Self::Private => "private",
		}
	}

	/// The label suffix a subnet of this tier in availability zone `zone`
	/// composes from, ie `private-b`.
	pub fn kind(&self, zone: &str) -> String {
		format!("{}-{zone}", self.label())
	}

	/// The third octet the tier's first subnet takes, spaced far enough apart
	/// that a third availability zone is an append rather than a renumber.
	fn octet(&self) -> usize {
		match self {
			Self::Public => 0,
			Self::Private => 10,
		}
	}
}

/// The network a block sits in: the source half of the [`VpcConsumers`]
/// relationship, on the consumer's entity, targeting a declaration carrying a
/// [`VpcBlock`]. Authored in markup as `{VpcRef($net)}` beside a
/// `<VpcBlock bx:ref="net"/>`.
///
/// The consumer's render system reads the [`VpcBlock`] off the target and asks
/// it for the terraform addresses, so both sides of every reference are one
/// composition rather than two that agree until one is renamed.
#[derive(Debug, Clone, PartialEq, Eq, Reflect, Component)]
#[reflect(Component)]
#[relationship(relationship_target = VpcConsumers)]
pub struct VpcRef(#[entities] pub Entity);

/// Every block sitting in a vpc: the target half of the [`VpcRef`]
/// relationship, on the vpc's declaration entity.
#[derive(Debug, Default, Reflect, Component)]
#[reflect(Component)]
#[relationship_target(relationship = VpcRef)]
pub struct VpcConsumers(Vec<Entity>);

#[cfg(test)]
mod tests {
	use super::*;
	use serde_json::Value;

	/// The config `block` emits against a Sydney stack, ie the one the mail
	/// stack deploys into and the one whose availability zones the subnets name.
	fn build_config(block: &VpcBlock) -> (ResolvedStack, terra::Config) {
		let (scope, _dir) = RenderScope::test_render_stack(
			Stack::new("beet_infra").with_region(aws::region::AP_SOUTHEAST_2),
			|parent| {
				parent.spawn(block.clone());
			},
		);
		let (stack, _deployment, config) = scope.finish().unwrap();
		(stack, config)
	}

	/// Every resource of `resource_type` the config carries, keyed by terraform
	/// label.
	fn resources(
		config: &terra::Config,
		resource_type: &str,
	) -> serde_json::Map<String, Value> {
		let Some(Value::Object(resources)) = config
			.to_json()
			.into_json()
			.get("resource")
			.and_then(|it| it.get(resource_type))
			.cloned()
		else {
			return default();
		};
		resources
	}

	/// The network is a spec like the dns table is: two tiers across two zones,
	/// each `/24` derived from the vpc's `/16` rather than written out, so a
	/// renumbered vpc cannot leave a subnet behind in the old range.
	#[beet_core::test]
	fn subnets_span_two_tiers_and_two_zones() {
		let (_stack, config) = build_config(&VpcBlock::new("net"));
		let mut subnets = resources(&config, "aws_subnet")
			.values()
			.map(|subnet| {
				format!(
					"{} {} {}",
					subnet["tags"]["Name"].as_str().unwrap(),
					subnet["cidr_block"].as_str().unwrap(),
					subnet["availability_zone"].as_str().unwrap(),
				)
			})
			.collect::<Vec<_>>();
		subnets.sort();
		subnets.xpect_eq(vec![
			"net--private-a 10.0.10.0/24 ap-southeast-2a",
			"net--private-b 10.0.11.0/24 ap-southeast-2b",
			"net--public-a 10.0.0.0/24 ap-southeast-2a",
			"net--public-b 10.0.1.0/24 ap-southeast-2b",
		]);
	}

	/// Only the public subnets take a public address at launch. A private
	/// subnet that quietly flipped this would put the database on the internet
	/// with nothing but a security group between.
	#[beet_core::test]
	fn only_public_subnets_map_public_ips() {
		let (_stack, config) = build_config(&VpcBlock::new("net"));
		let mut mapped = resources(&config, "aws_subnet")
			.values()
			.map(|subnet| {
				(
					subnet["tags"]["Name"].as_str().unwrap().to_string(),
					subnet["map_public_ip_on_launch"].as_bool().unwrap(),
				)
			})
			.collect::<Vec<_>>();
		mapped.sort();
		mapped.xpect_eq(vec![
			("net--private-a".to_string(), false),
			("net--private-b".to_string(), false),
			("net--public-a".to_string(), true),
			("net--public-b".to_string(), true),
		]);
	}

	/// No NAT gateway, and exactly one route table: the private subnets are
	/// left on the vpc's main table, whose only route is the local cidr. The
	/// two associations are both public ones.
	#[beet_core::test]
	fn private_subnets_have_no_egress() {
		let (stack, config) = build_config(&VpcBlock::new("net"));
		let json = config.to_json_string().unwrap();
		json.as_str().xnot().xpect_contains("aws_nat_gateway");
		resources(&config, "aws_route_table").len().xpect_eq(1);
		let block = VpcBlock::new("net");
		let mut associated = resources(&config, "aws_route_table_association")
			.values()
			.map(|assoc| assoc["subnet_id"].as_str().unwrap().to_string())
			.collect::<Vec<_>>();
		associated.sort();
		associated.xpect_eq(vec![
			block.subnet_id(&stack, SubnetTier::Public, "a"),
			block.subnet_id(&stack, SubnetTier::Public, "b"),
		]);
	}

	/// The block's address compositions are the only way a consumer reaches
	/// these resources, so they must be the ones actually emitted. A drift here
	/// is an interpolation to a resource that does not exist, which terraform
	/// reports at apply rather than at plan.
	#[beet_core::test]
	fn composed_addresses_match_what_is_emitted() {
		let (stack, config) = build_config(&VpcBlock::new("net"));
		let block = VpcBlock::new("net");
		let address = |reference: &str| {
			reference
				.trim_start_matches("${")
				.rsplit_once('.')
				.unwrap()
				.0
				.to_string()
		};
		for (resource_type, reference) in [
			("aws_vpc", block.id(&stack)),
			(
				"aws_subnet",
				block.subnet_id(&stack, SubnetTier::Private, "a"),
			),
		] {
			let label = address(&reference)
				.trim_start_matches(&format!("{resource_type}."))
				.to_string();
			resources(&config, resource_type)
				.contains_key(&label)
				.xpect_true();
		}
		// ..and a subnet group's worth of them, in zone order
		block
			.subnet_ids(&stack, SubnetTier::Private)
			.len()
			.xpect_eq(2);
	}

	/// The cidr is the one field a caller can get wrong, and every subnet is
	/// derived from it, so a prefix that has no third octet to give away fails
	/// the apply rather than emitting overlapping `/24`s.
	#[beet_core::test]
	fn only_a_base_slash_16_is_accepted() {
		VpcBlock::new("net")
			.with_cidr("10.1.0.0/16")
			.network_prefix()
			.unwrap()
			.as_str()
			.xpect_eq("10.1");
		VpcBlock::new("net")
			.with_cidr("10.0.0.0/24")
			.network_prefix()
			.unwrap_err()
			.to_string()
			.xpect_contains("must be a /16");
		VpcBlock::new("net")
			.with_cidr("10.0.5.0/16")
			.network_prefix()
			.unwrap_err()
			.to_string()
			.xpect_contains("is not the base address");
	}
}
