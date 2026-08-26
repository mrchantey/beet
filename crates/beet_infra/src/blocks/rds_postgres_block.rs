use crate::bindings::*;
use crate::prelude::*;
use crate::terra::ResourceDef;
use beet_core::prelude::*;
use serde_json::json;

/// The company Postgres instance: one managed database in the private subnets
/// of a [`VpcBlock`], reachable only from the security groups that declare
/// themselves consumers of it.
///
/// One instance, many tenants. A database here is a logical database and a role
/// inside it, so a second application is a second tenant rather than a second
/// $19 a month; the instance is sized for the sum of them and grown when that
/// stops being true.
///
/// Durability is layered and none of it is optional: fourteen days of automated
/// backups (point-in-time recovery to about five minutes), a final snapshot
/// taken on delete, and deletion protection so `tofu destroy` refuses the
/// instance outright until somebody turns it off on purpose.
///
/// Authored directly from markup, ie
/// `<RdsPostgresBlock label="db" database="mail"/>`, with the vpc and the
/// consumers named by label through [`VpcRef`] and [`SecurityGroupRef`].
#[derive(
	Debug, Clone, Get, SetWith, Serialize, Deserialize, Component, Reflect,
)]
#[reflect(Component, Default)]
#[component(immutable, on_add = ErasedBlock::on_add::<RdsPostgresBlock>)]
pub struct RdsPostgresBlock {
	label: SmolStr,
	/// The network this instance lives in. Its PRIVATE subnets, always: an
	/// instance in the public ones is one security-group edit from the internet.
	vpc: VpcRef,
	/// The security groups admitted to [`PORT`](Self::PORT), and the only
	/// things that can reach the instance at all. Declared rather than derived,
	/// so adding a consumer is a line of markup and never a console click.
	#[set_with(skip)]
	consumers: Vec<SecurityGroupRef>,
	instance_class: SmolStr,
	/// The major version, pinned. Minor upgrades happen in the maintenance
	/// window; a major one is a deliberate edit.
	engine_version: SmolStr,
	/// Gigabytes at launch. Storage autoscales up to
	/// [`max_storage`](Self::max_storage), so this is a floor rather than a
	/// guess that has to be right.
	storage: i64,
	max_storage: i64,
	/// The logical database created with the instance, ie the first tenant.
	database: SmolStr,
	/// The master role. Not an application's role: tenants get their own.
	username: SmolStr,
	/// The master password, arriving as a tofu variable rather than a literal.
	/// Sensitive, so tofu redacts it everywhere except state, which is why a
	/// stack declaring one runs with state encryption on.
	#[set_with(skip)]
	password: Variable,
	/// Override where the master password is stored, which otherwise composes
	/// from the stack. `EnsureSecret` creates it and the deploy reads
	/// [`password`](Self::password) back out of it; the running consumer reads
	/// it directly, which is what [`Block::runtime_access`] grants.
	#[get(skip)]
	#[set_with(unwrap_option, into)]
	secret: Option<SmolStr>,
	/// Days of automated backups, ie the point-in-time recovery window.
	backup_days: i64,
	/// Refuse to delete the instance. The one setting between a mistyped
	/// `destroy` and every mailbox in the company.
	deletion_protection: bool,
	/// Add a tofu output for the instance endpoint.
	output: bool,
}

impl Default for RdsPostgresBlock {
	fn default() -> Self { Self::new("", "") }
}

impl RdsPostgresBlock {
	/// Postgres. Fixed by the engine, and named here so a security group rule
	/// and a connection string cannot disagree about it.
	pub const PORT: i64 = 5432;
	pub const ENGINE: &'static str = "postgres";
	/// The smallest Graviton instance, which is the right size for a mail
	/// server's metadata and has room for the tenants that follow.
	pub const INSTANCE_CLASS: &'static str = "db.t4g.micro";
	pub const ENGINE_VERSION: &'static str = "17";
	/// gp3 rather than gp2: same price, better baseline throughput, and it does
	/// not tie iops to volume size.
	pub const STORAGE_TYPE: &'static str = "gp3";
	/// The label suffix of the instance itself, ie the `db--db` in a stack
	/// whose vpc is `net--vpc`.
	pub const INSTANCE: &'static str = "db";
	pub const SUBNET_GROUP: &'static str = "subnets";
	/// The [`AccessGrant::kind`] for an SSM parameter, ie the master password
	/// this block tucks away for a consumer to read at boot.
	pub const ACCESS_KIND: &'static str = "ssm_parameter";

	/// An instance holding `database` as its first tenant, in the vpc labelled
	/// `vpc`. It has no consumers yet, which is a declaration that will not
	/// apply: see [`with_consumer`](Self::with_consumer).
	pub fn new(label: impl Into<SmolStr>, vpc: impl Into<SmolStr>) -> Self {
		let label = label.into();
		Self {
			vpc: VpcRef::new(vpc),
			consumers: Vec::new(),
			instance_class: Self::INSTANCE_CLASS.into(),
			engine_version: Self::ENGINE_VERSION.into(),
			storage: 20,
			max_storage: 100,
			database: label.clone(),
			username: "postgres".into(),
			password: Variable::param(format!("{label}_password"))
				.with_sensitive(true),
			secret: None,
			backup_days: 14,
			deletion_protection: true,
			output: true,
			label,
		}
	}

	/// Admit one security group to the instance.
	pub fn with_consumer(mut self, consumer: SecurityGroupRef) -> Self {
		self.consumers.push(consumer);
		self
	}

	/// The handle a consumer composes its connection from.
	pub fn database_ref(&self) -> DatabaseRef {
		DatabaseRef::new(self.label.clone())
	}

	/// This instance's own security group, the one the consumers' rules point
	/// at.
	pub fn security_group(&self) -> SecurityGroupRef {
		SecurityGroupRef::new(self.label.clone())
	}

	/// The SSM parameter the master password lives in, ie
	/// `/beetmash/prod/db-password`. The stack's own `app--stage--label`
	/// composition with its separators as slashes, so parameter store nests the
	/// stack's secrets under a prefix an IAM policy can grant in one statement.
	pub fn secret_name(&self, stack: &ResolvedStack) -> String {
		self.secret.clone().map(Into::into).unwrap_or_else(|| {
			format!(
				"/{}",
				stack
					.resource_name(format!("{}-password", self.label))
					.replace("--", "/")
			)
		})
	}

	/// Names for RDS, which does not take the usual `app--stage--label`: an
	/// instance identifier may not contain two consecutive hyphens, so the
	/// separator collapses to one. Applied to the subnet group too, so the two
	/// read as one instance in the console rather than as two conventions.
	pub fn rds_name(&self, stack: &ResolvedStack, kind: &str) -> String {
		stack
			.resource_name(format!("{}--{kind}", self.label))
			.replace("--", "-")
	}

	/// Reject a declaration that cannot apply, at config time: an unreachable
	/// instance, or a name the engine will refuse after twenty minutes of
	/// provisioning.
	pub fn validate(&self) -> Result {
		if self.consumers.is_empty() {
			bevybail!(
				"database '{}' admits no security group, so nothing can reach it: name its consumers with `with_consumer`",
				self.label
			);
		}
		Self::validate_identifier(&self.database, "database name")?;
		Self::validate_identifier(&self.username, "master username")?;
		if self.max_storage < self.storage {
			bevybail!(
				"database '{}' autoscales to {}GB, which is below the {}GB it starts at",
				self.label,
				self.max_storage,
				self.storage
			);
		}
		Ok(())
	}

	/// Postgres identifiers start with a letter and carry only lowercase
	/// alphanumerics and underscores. Anything else is quoted-identifier
	/// territory, which a connection string in a config file will not survive.
	fn validate_identifier(value: &str, field: &str) -> Result {
		let valid = value
			.chars()
			.next()
			.is_some_and(|char| char.is_ascii_lowercase())
			&& value.chars().all(|char| {
				char.is_ascii_lowercase()
					|| char.is_ascii_digit()
					|| char == '_'
			});
		if !valid {
			bevybail!(
				"{field} '{value}' must start with a lowercase letter and hold only lowercase letters, digits and underscores"
			);
		}
		Ok(())
	}

	fn tags(
		&self,
		stack: &ResolvedStack,
		kind: &str,
	) -> std::collections::BTreeMap<SmolStr, SmolStr> {
		[
			(
				SmolStr::from("Name"),
				format!("{}--{kind}", self.label).as_str().into(),
			),
			(SmolStr::from("Project"), stack.app_name().clone()),
			(SmolStr::from("Stage"), stack.stage().clone()),
		]
		.into_iter()
		.collect()
	}
}

impl Block for RdsPostgresBlock {
	fn apply_to_config(
		&self,
		_entity: &EntityRef,
		stack: &ResolvedStack,
		_deployment: &Deployment,
		_access: &AccessGrants,
		config: &mut terra::Config,
	) -> Result {
		self.validate()?;
		config.ensure_variable(
			self.password.key().as_str(),
			self.password.tf_declaration(),
		);
		let group = self.emit_security_group(stack, config)?;
		self.emit_instance(stack, config, &group)?;
		Ok(())
	}

	/// What a process running beside this instance does with it: read the
	/// master password out of the parameter it was put in. The connection
	/// itself needs no cloud permission at all, being a tcp session inside the
	/// vpc that the security group either admits or does not.
	fn runtime_access(&self, stack: &ResolvedStack) -> Vec<AccessGrant> {
		vec![AccessGrant::read(Self::ACCESS_KIND, self.secret_name(stack))]
	}

	fn variables(&self) -> &[Variable] { std::slice::from_ref(&self.password) }
}

impl RdsPostgresBlock {
	/// The instance's security group and one ingress rule per consumer.
	///
	/// No egress rule at all, which is not an omission: declaring a group with
	/// rules of its own replaces the default allow-everything egress, and a
	/// database has nothing to call out to.
	fn emit_security_group(
		&self,
		stack: &ResolvedStack,
		config: &mut terra::Config,
	) -> Result<ResourceDef<AwsSecurityGroupDetails>> {
		let group = ResourceDef::new_primary(
			self.security_group().ident(stack),
			AwsSecurityGroupDetails {
				description: Some(
					format!("Postgres access for {}", self.label).into(),
				),
				vpc_id: Some(self.vpc.id(stack).into()),
				tags: Some(self.tags(stack, SecurityGroupRef::KIND)),
				..default()
			},
		);
		config.add_resource(&group)?;
		for consumer in &self.consumers {
			config.add_resource(&ResourceDef::new_secondary(
				stack.resource_ident(format!(
					"{}--sg-from-{}",
					self.label,
					consumer.label()
				)),
				AwsSecurityGroupRuleDetails {
					security_group_id: group.field_ref("id").into(),
					r#type: "ingress".into(),
					from_port: Self::PORT,
					to_port: Self::PORT,
					protocol: "tcp".into(),
					// the consumer's group, never a cidr: an address range
					// admits whatever happens to be in it later.
					source_security_group_id: Some(consumer.id(stack).into()),
					description: Some(
						format!("Postgres from {}", consumer.label()).into(),
					),
					..default()
				},
			))?;
		}
		group.xok()
	}

	/// The subnet group spanning the vpc's private subnets, and the instance
	/// itself.
	fn emit_instance(
		&self,
		stack: &ResolvedStack,
		config: &mut terra::Config,
		group: &ResourceDef<AwsSecurityGroupDetails>,
	) -> Result {
		let subnets = ResourceDef::new_secondary(
			stack.resource_ident(format!(
				"{}--{}",
				self.label,
				Self::SUBNET_GROUP
			)),
			AwsDbSubnetGroupDetails {
				name: Some(self.rds_name(stack, Self::SUBNET_GROUP).into()),
				subnet_ids: self.vpc.subnet_ids(stack, SubnetTier::Private),
				tags: Some(self.tags(stack, Self::SUBNET_GROUP)),
				..default()
			},
		);
		let identifier = self.rds_name(stack, Self::INSTANCE);
		let instance = ResourceDef::new_secondary(
			stack.resource_ident(format!("{}--{}", self.label, Self::INSTANCE)),
			AwsDbInstanceDetails {
				identifier: Some(identifier.clone().into()),
				engine: Some(Self::ENGINE.into()),
				engine_version: Some(self.engine_version.clone()),
				instance_class: self.instance_class.clone(),
				allocated_storage: Some(self.storage),
				// autoscaling, so a full disk is a bill rather than an outage
				max_allocated_storage: Some(self.max_storage),
				storage_type: Some(Self::STORAGE_TYPE.into()),
				storage_encrypted: Some(true),
				db_name: Some(self.database.clone()),
				username: Some(self.username.clone()),
				password: Some(self.password.tf_var_ref().into()),
				port: Some(Self::PORT),
				db_subnet_group_name: Some(subnets.field_ref("name").into()),
				vpc_security_group_ids: Some(vec![
					group.field_ref("id").into(),
				]),
				// private subnets and no public address: the security group is
				// the second lock, not the first.
				publicly_accessible: Some(false),
				multi_az: Some(false),
				backup_retention_period: Some(self.backup_days),
				copy_tags_to_snapshot: Some(true),
				auto_minor_version_upgrade: Some(true),
				// a fixed name, so a plan shows no diff between deploys. The
				// cost is that a destroyed-and-rebuilt instance must have its
				// old final snapshot renamed before it can be destroyed again.
				skip_final_snapshot: Some(false),
				final_snapshot_identifier: Some(
					format!("{identifier}-final").into(),
				),
				deletion_protection: Some(self.deletion_protection),
				tags: Some(self.tags(stack, Self::INSTANCE)),
				..default()
			},
		);
		config.add_resource(&subnets)?.add_resource(&instance)?;
		if self.output {
			config.add_output(
				format!("{}_endpoint", self.label),
				terra::Output {
					value: json!(instance.field_ref("endpoint")),
					description: Some(
						format!("The postgres endpoint for {}", self.label)
							.into(),
					),
					sensitive: None,
				},
			)?;
		}
		Ok(())
	}
}

/// Names an [`RdsPostgresBlock`] declared in the same stack, and composes the
/// terraform references a consumer builds its connection string from.
///
/// The counterpart of [`VpcRef`]: the block emits under the idents this hands
/// out, so a consumer's connection string and the instance it points at cannot
/// drift.
#[derive(
	Debug, Default, Clone, Get, Serialize, Deserialize, PartialEq, Eq, Reflect,
)]
pub struct DatabaseRef {
	label: SmolStr,
}

impl DatabaseRef {
	pub fn new(label: impl Into<SmolStr>) -> Self {
		Self {
			label: label.into(),
		}
	}

	pub fn ident(&self, stack: &ResolvedStack) -> terra::Ident {
		stack.resource_ident(format!(
			"{}--{}",
			self.label,
			RdsPostgresBlock::INSTANCE
		))
	}

	fn field_ref(&self, stack: &ResolvedStack, field: &str) -> String {
		format!("${{aws_db_instance.{}.{field}}}", self.ident(stack).label())
	}

	/// The instance's hostname, without a port.
	pub fn host(&self, stack: &ResolvedStack) -> String {
		self.field_ref(stack, "address")
	}

	pub fn port(&self, stack: &ResolvedStack) -> String {
		self.field_ref(stack, "port")
	}

	/// `host:port`, which is what a connection string wants.
	pub fn endpoint(&self, stack: &ResolvedStack) -> String {
		self.field_ref(stack, "endpoint")
	}

	/// The group a consumer must be admitted to, ie the one an
	/// [`RdsPostgresBlock`] declares for itself.
	pub fn security_group(&self) -> SecurityGroupRef {
		SecurityGroupRef::new(self.label.clone())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use serde_json::Value;

	/// The mail stack's database as the plan declares it: the box's security
	/// group is its one consumer.
	fn database() -> RdsPostgresBlock {
		RdsPostgresBlock::new("db", "net")
			.with_database("mail")
			.with_consumer(SecurityGroupRef::new("mail"))
	}

	/// The config `block` emits against a Sydney stack.
	fn build_config(
		block: &RdsPostgresBlock,
	) -> (ResolvedStack, terra::Config) {
		let (stack, deployment, _dir) = ResolvedStack::default_local();
		let stack = stack.with_region(aws::region::AP_SOUTHEAST_2);
		let mut config = deployment.create_config(&stack);
		let mut world = World::new();
		block
			.apply_to_config(
				&world.spawn(()).as_readonly(),
				&stack,
				&deployment,
				&default(),
				&mut config,
			)
			.unwrap();
		(stack, config)
	}

	/// The sole `aws_db_instance` the config carries.
	fn instance(config: &terra::Config) -> Value {
		config.to_json()["resource"]["aws_db_instance"]
			.as_object()
			.unwrap()
			.values()
			.next()
			.unwrap()
			.clone()
	}

	/// Every setting between a mistyped command and the loss of every mailbox
	/// in the company, pinned. None of these is a default: RDS ships deletion
	/// protection off, backups at one day and `skip_final_snapshot` unset.
	#[beet_core::test]
	fn durability_settings_are_not_defaults() {
		let instance = instance(&build_config(&database()).1);
		instance["deletion_protection"]
			.as_bool()
			.unwrap()
			.xpect_true();
		instance["backup_retention_period"]
			.as_i64()
			.unwrap()
			.xpect_eq(14);
		instance["skip_final_snapshot"]
			.as_bool()
			.unwrap()
			.xpect_false();
		instance["final_snapshot_identifier"]
			.as_str()
			.unwrap()
			.xpect_contains("-final");
		instance["storage_encrypted"]
			.as_bool()
			.unwrap()
			.xpect_true();
	}

	/// An RDS identifier may not contain two consecutive hyphens, which is
	/// exactly what `app--stage--label` is. The failure is at apply, twenty
	/// minutes in, so it is worth pinning the collapsed form here.
	#[beet_core::test]
	fn rds_names_collapse_the_double_hyphen() {
		let (stack, config) = build_config(&database());
		// the ordinary convention, for comparison: `app--stage--label`
		stack
			.resource_name("db--db")
			.as_str()
			.xpect_eq("beet-infra--dev--db-db");
		let instance = instance(&config);
		instance["identifier"]
			.as_str()
			.unwrap()
			.xpect_eq("beet-infra-dev-db-db")
			.xnot()
			.xpect_contains("--");
		config.to_json()["resource"]["aws_db_subnet_group"]
			.as_object()
			.unwrap()
			.values()
			.next()
			.unwrap()["name"]
			.as_str()
			.unwrap()
			.xnot()
			.xpect_contains("--");
	}

	/// The instance is in the vpc's PRIVATE subnets and takes no public
	/// address. Either half of this going wrong puts a database holding every
	/// message in the company on the internet.
	#[beet_core::test]
	fn instance_is_private() {
		let (stack, config) = build_config(&database());
		instance(&config)["publicly_accessible"]
			.as_bool()
			.unwrap()
			.xpect_false();
		let vpc = VpcBlock::new("net").vpc_ref();
		config.to_json()["resource"]["aws_db_subnet_group"]
			.as_object()
			.unwrap()
			.values()
			.next()
			.unwrap()["subnet_ids"]
			.as_array()
			.unwrap()
			.iter()
			.map(|id| id.as_str().unwrap().to_string())
			.collect::<Vec<_>>()
			.xpect_eq(
				vpc.subnet_ids(&stack, SubnetTier::Private)
					.into_iter()
					.map(|id| id.to_string())
					.collect::<Vec<_>>(),
			);
	}

	/// Reachable from the declared consumers and from nothing else: one ingress
	/// rule on the postgres port sourced from a security group, no cidr
	/// anywhere, and no egress rule at all.
	#[beet_core::test]
	fn only_declared_consumers_are_admitted() {
		let (stack, config) = build_config(&database());
		let rules = config.to_json()["resource"]["aws_security_group_rule"]
			.as_object()
			.unwrap()
			.values()
			.cloned()
			.collect::<Vec<_>>();
		rules.len().xpect_eq(1);
		let rule = &rules[0];
		rule["type"].as_str().unwrap().xpect_eq("ingress");
		rule["from_port"].as_i64().unwrap().xpect_eq(5432);
		rule["to_port"].as_i64().unwrap().xpect_eq(5432);
		rule["source_security_group_id"]
			.as_str()
			.unwrap()
			.xpect_eq(SecurityGroupRef::new("mail").id(&stack));
		rule["cidr_blocks"].is_null().xpect_true();
		config
			.to_json()
			.to_string()
			.as_str()
			.xnot()
			.xpect_contains("\"egress\"");
	}

	/// A database nothing may reach is a typo, not a configuration, so it fails
	/// the apply rather than provisioning twenty minutes of unreachable
	/// instance.
	#[beet_core::test]
	fn a_database_with_no_consumer_is_an_error() {
		RdsPostgresBlock::new("db", "net")
			.validate()
			.unwrap_err()
			.to_string()
			.xpect_contains("admits no security group");
		RdsPostgresBlock::new("db", "net")
			.with_database("Mail-DB")
			.with_consumer(SecurityGroupRef::new("mail"))
			.validate()
			.unwrap_err()
			.to_string()
			.xpect_contains("database name 'Mail-DB'");
	}

	/// The password is a sensitive tofu variable and never a literal: the
	/// rendered config holds the reference, and the declaration is marked so
	/// tofu redacts it in plan and apply output. State is not redacted, which
	/// is what state encryption is for.
	#[beet_core::test]
	fn the_password_is_a_sensitive_variable() {
		let (_stack, config) = build_config(&database());
		let json = config.to_json();
		instance(&config)["password"]
			.as_str()
			.unwrap()
			.xpect_eq("${var.db_password}");
		json["variable"]["db_password"]["sensitive"]
			.as_bool()
			.unwrap()
			.xpect_true();
		// ..and the deploy knows to resolve it, so `apply` passes a `-var`
		database().variables().len().xpect_eq(1);
	}

	/// The one permission a process beside this instance needs, which is to
	/// read the password out of the parameter it was put in. The connection
	/// itself is a tcp session the security group either admits or does not.
	#[beet_core::test]
	fn grants_read_on_its_own_secret() {
		let (stack, _config) = build_config(&database());
		database()
			.runtime_access(&stack)
			.xpect_eq(vec![AccessGrant::read(
				RdsPostgresBlock::ACCESS_KIND,
				"/beet-infra/dev/db-password",
			)]);
	}

	/// The rendered pair, through the real provider. Rendered-json assertions
	/// prove what this block MEANT to emit; only tofu proves the schema accepts
	/// it, and only a run holding both blocks proves the database's
	/// interpolations reach subnets the vpc actually declared.
	///
	/// Drives the native tofu cli, so it cannot compile for wasm.
	#[cfg(not(target_arch = "wasm32"))]
	#[beet_core::test(timeout_ms = 120000)]
	#[ignore = "very slow"]
	async fn validate() {
		let (stack, deployment, _dir) = ResolvedStack::default_local();
		let stack = stack.with_region(aws::region::AP_SOUTHEAST_2);
		let network = VpcBlock::new("net");
		let db = database();
		let mut world = World::new();
		let spawned = world.spawn(());
		let entity = spawned.as_readonly();
		let mut config = stack
			.build_config(&deployment, [
				(entity.clone(), &network as &dyn Block),
				(entity, &db as &dyn Block),
			])
			.unwrap();
		// the consumer's own group, which belongs to whichever block owns the
		// box: this stack has none, so it stands in for one. Its label is
		// composed by the same `SecurityGroupRef` the ingress rule read, which
		// is the half of the reference under test.
		config
			.add_untyped_resource(
				"aws_security_group",
				SecurityGroupRef::new("mail").ident(&stack).label(),
				&json!({ "name": "consumer", "vpc_id": VpcBlock::new("net").vpc_ref().id(&stack) }),
			)
			.unwrap();
		terra::Project::new(stack, deployment, config)
			.validate()
			.await
			.unwrap();
	}

	/// A [`DatabaseRef`] is how phase-later compute blocks compose a connection
	/// string, so the address it builds must be the instance actually emitted.
	#[beet_core::test]
	fn database_ref_points_at_the_emitted_instance() {
		let (stack, config) = build_config(&database());
		let label = database().database_ref().ident(&stack).label().to_string();
		config.to_json()["resource"]["aws_db_instance"]
			.as_object()
			.unwrap()
			.contains_key(label.as_str())
			.xpect_true();
		database()
			.database_ref()
			.host(&stack)
			.as_str()
			.xpect_eq(format!("${{aws_db_instance.{label}.address}}"));
		// ..and the group a consumer must be admitted to is the one declared
		database()
			.database_ref()
			.security_group()
			.xpect_eq(database().security_group());
	}
}
