use crate::bindings::*;
use crate::prelude::*;
use crate::terra::ResourceDef;
use beet_core::prelude::*;

/// A small private network: one vpc, a public and optionally a private subnet in
/// each declared availability zone, and an internet gateway the public side
/// routes through.
///
/// The topology is DECLARED rather than assumed, because the right one is a
/// property of the workloads and nothing here can infer it: `<VpcBlock
/// zones={["a"]} private_tier=false/>` is one subnet for a single public box,
/// the default two zones with both tiers is four and is what an RDS subnet
/// group requires, and three zones with both tiers is six. A stack pays for
/// none of them (subnets are free) but each is a resource in its plan, and a
/// private tier nothing sits in is a tier somebody will later wonder about.
///
/// Deliberately no NAT gateway. The private subnets stay on the vpc's main
/// route table, whose only route is the local cidr, so a private instance has
/// no path off the vpc at all. Giving it one costs about $32 a month, and the
/// resource this tier exists for (a database) has nothing to reach out to.
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
#[component(immutable, on_insert = ErasedBlock::on_insert::<Self>,
	on_remove = ErasedBlock::on_remove
)]
pub struct VpcBlock {
	label: SmolStr,
	/// The network this vpc owns, which must be a `/16`: every subnet is a
	/// `/24` carved out of its third octet.
	cidr: SmolStr,
	/// The availability zones this vpc spans, as region suffixes, ie
	/// `["a", "b"]` for `ap-southeast-2a` and `ap-southeast-2b`. One subnet per
	/// tier per zone, in this order: the `index`th zone takes the `index`th
	/// `/24` of its tier, so appending a zone never renumbers an existing
	/// subnet and reordering the list renumbers all of them.
	zones: Vec<SmolStr>,
	/// Whether the private tier exists. On by default because the blocks that
	/// consume this network mostly want one; a stack whose only workload is a
	/// public box declares `false` and emits half as many subnets.
	private_tier: bool,
}

impl Default for VpcBlock {
	fn default() -> Self { Self::new("") }
}

impl VpcBlock {
	/// The default network. Private space, and a `/16` so the third octet is
	/// free for subnets to number themselves with.
	pub const CIDR: &'static str = "10.0.0.0/16";

	/// The zones a vpc spans unless it declares otherwise. Two, because that is
	/// the minimum an RDS subnet group accepts and every region has an `a` and
	/// a `b`.
	pub const ZONES: &'static [&'static str] = &["a", "b"];

	/// The most zones one tier can hold before its `/24`s would collide with
	/// the next tier's, ie the spacing in [`SubnetTier::octet`].
	pub const MAX_ZONES: usize = 10;

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
			zones: Self::ZONES.iter().copied().map(SmolStr::from).collect(),
			private_tier: true,
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

	/// One subnet's availability zone, as a reference rather than as the literal
	/// this block composes it from.
	///
	/// An EBS volume must be created in the zone of the instance it attaches to,
	/// so a consumer declaring one has to name that zone. Naming it by reference
	/// is what stops the two from ever disagreeing: a vpc that renumbers its
	/// zones moves the volumes with it instead of failing an attachment.
	pub fn subnet_availability_zone(
		&self,
		stack: &ResolvedStack,
		tier: SubnetTier,
		zone: &str,
	) -> String {
		self.field_ref(
			stack,
			"aws_subnet",
			&tier.kind(zone),
			"availability_zone",
		)
	}

	/// Every subnet id of one tier, in availability-zone order. What a db
	/// subnet group or a load balancer is spread across.
	pub fn subnet_ids(
		&self,
		stack: &ResolvedStack,
		tier: SubnetTier,
	) -> Vec<SmolStr> {
		self.zones
			.iter()
			.map(|zone| self.subnet_id(stack, tier, zone).into())
			.collect()
	}

	/// The tiers this vpc actually emits, public first.
	pub fn tiers(&self) -> Vec<SubnetTier> {
		match self.private_tier {
			true => vec![SubnetTier::Public, SubnetTier::Private],
			false => vec![SubnetTier::Public],
		}
	}

	/// Whether this vpc emits `tier` at all, ie what a consumer needing a
	/// private subnet asks before composing a reference to one.
	pub fn has_tier(&self, tier: SubnetTier) -> bool {
		self.tiers().contains(&tier)
	}

	/// Rejects a topology that cannot be emitted, at config time.
	///
	/// A zone list that is empty, repeats itself or outruns the `/24` spacing
	/// between tiers would otherwise become a duplicate ident or a silently
	/// overlapping cidr, both of which surface as an apply-time AWS error a
	/// long way from the declaration that caused them.
	pub fn validate(&self) -> Result {
		if self.zones.is_empty() {
			bevybail!(
				"vpc '{}' declares no availability zones: a network with no subnet has nothing to put in it, ie `zones={{[\"a\"]}}`",
				self.label
			);
		}
		if self.zones.len() > Self::MAX_ZONES {
			bevybail!(
				"vpc '{}' declares {} availability zones, more than the {} its /24 tier spacing allows",
				self.label,
				self.zones.len(),
				Self::MAX_ZONES
			);
		}
		let mut seen = HashSet::<&SmolStr>::default();
		for zone in &self.zones {
			if !seen.insert(zone) {
				bevybail!(
					"vpc '{}' declares availability zone '{zone}' twice, so two subnets would compose the same ident",
					self.label
				);
			}
		}
		self.network_prefix()?;
		Ok(())
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
		self.validate()?;
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
		for tier in self.tiers() {
			for (index, zone) in self.zones.iter().enumerate() {
				let kind = tier.kind(zone);
				config.add_resource(&ResourceDef::new_secondary(
					self.ident(stack, &kind),
					AwsSubnetDetails {
						vpc_id: vpc.field_ref("id").into(),
						cidr_block: Some(self.subnet_cidr(tier, index)?.into()),
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
		for zone in &self.zones {
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

	/// The declared topology is what gets emitted, and the default is unchanged:
	/// a zone list of one drops to a public subnet alone, three zones with both
	/// tiers is six, and a zone's `/24` is its POSITION in the list, so
	/// appending never renumbers an existing subnet.
	#[beet_core::test]
	fn the_topology_follows_the_declaration() {
		let names = |block: &VpcBlock| {
			let (_stack, config) = build_config(block);
			let mut subnets = resources(&config, "aws_subnet")
				.values()
				.map(|subnet| {
					format!(
						"{} {}",
						subnet["tags"]["Name"].as_str().unwrap(),
						subnet["cidr_block"].as_str().unwrap(),
					)
				})
				.collect::<Vec<_>>();
			subnets.sort();
			subnets
		};
		// one public box wants one subnet
		names(
			&VpcBlock::new("net")
				.with_private_tier(false)
				.with_zones(vec!["a".into()]),
		)
		.xpect_eq(vec!["net--public-a 10.0.0.0/24"]);
		// ..and a third zone appends rather than renumbering
		names(&VpcBlock::new("net").with_zones(
			["a", "b", "c"].into_iter().map(SmolStr::from).collect(),
		))
		.xpect_eq(vec![
			"net--private-a 10.0.10.0/24",
			"net--private-b 10.0.11.0/24",
			"net--private-c 10.0.12.0/24",
			"net--public-a 10.0.0.0/24",
			"net--public-b 10.0.1.0/24",
			"net--public-c 10.0.2.0/24",
		]);
		// every public subnet keeps its route-table association
		let (_stack, config) = build_config(&VpcBlock::new("net").with_zones(
			["a", "b", "c"].into_iter().map(SmolStr::from).collect(),
		));
		resources(&config, "aws_route_table_association")
			.len()
			.xpect_eq(3);
	}

	/// A topology that cannot be emitted fails at config time rather than
	/// becoming a duplicate ident or an overlapping cidr AWS reports at apply.
	#[beet_core::test]
	fn an_unemittable_topology_fails_at_config_time() {
		VpcBlock::new("net")
			.with_zones(Vec::new())
			.validate()
			.unwrap_err()
			.to_string()
			.xpect_contains("declares no availability zones");
		VpcBlock::new("net")
			.with_zones(["a", "a"].into_iter().map(SmolStr::from).collect())
			.validate()
			.unwrap_err()
			.to_string()
			.xpect_contains("twice");
		VpcBlock::new("net")
			.with_zones(
				(b'a'..=b'k')
					.map(|zone| SmolStr::from((zone as char).to_string()))
					.collect(),
			)
			.validate()
			.unwrap_err()
			.to_string()
			.xpect_contains("tier spacing allows");
		VpcBlock::new("net").validate().unwrap();
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
