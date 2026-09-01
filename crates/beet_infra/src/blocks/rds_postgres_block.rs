use crate::bindings::*;
use crate::prelude::*;
use crate::terra::ResourceDef;
use beet_core::prelude::*;
use serde_json::json;

/// The company Postgres instance: one managed database in the private subnets
/// of a [`VpcBlock`], reachable only from the security groups of the blocks
/// that relate themselves to it with a [`DatabaseRef`].
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
/// `<RdsPostgresBlock bx:ref="db" label="db" database="mail" {VpcRef($net)}/>`:
/// the network it lives in rides a [`VpcRef`] relation (its PRIVATE subnets,
/// always: an instance in the public ones is one security-group edit from the
/// internet), and each consumer admits itself by relating a [`DatabaseRef`] to
/// this declaration, so adding one is a line of markup and never a console
/// click.
#[derive(
	Debug, Clone, Get, SetWith, Serialize, Deserialize, Component, Reflect,
)]
#[reflect(Component, Default)]
#[component(immutable, on_insert = ErasedBlock::on_insert::<Self>,
	on_remove = ErasedBlock::on_remove
)]
pub struct RdsPostgresBlock {
	label: SmolStr,
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
	/// Override where the master password is stored, which otherwise composes
	/// from the stack. `EnsureSecret` creates it and the deploy reads
	/// [`password`](Self::password) back out of it; the running consumer reads
	/// it directly, which is what [`runtime_access`](Self::runtime_access) grants.
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
	fn default() -> Self { Self::new("") }
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

	/// An instance whose first tenant shares its label. Relate it to its network
	/// with a [`VpcRef`]; it has no consumers yet, which is a declaration that
	/// will not render: see [`DatabaseRef`].
	pub fn new(label: impl Into<SmolStr>) -> Self {
		let label = label.into();
		Self {
			instance_class: Self::INSTANCE_CLASS.into(),
			engine_version: Self::ENGINE_VERSION.into(),
			storage: 20,
			max_storage: 100,
			database: label.clone(),
			username: "postgres".into(),
			secret: None,
			backup_days: 14,
			deletion_protection: true,
			output: true,
			label,
		}
	}

	/// The terraform ident of the instance itself, ie what a consumer's
	/// connection interpolations resolve against.
	pub fn ident(&self, stack: &ResolvedStack) -> terra::Ident {
		stack.resource_ident(format!("{}--{}", self.label, Self::INSTANCE))
	}

	/// An interpolated reference to `field` of the instance.
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

	/// The master password, arriving as a tofu variable rather than a literal.
	/// Sensitive, so tofu redacts it everywhere except state, which is why a
	/// stack declaring one runs with state encryption on.
	///
	/// Derived from the label rather than stored, so it cannot disagree with
	/// the [`password_variable`](Self::password_variable) `EnsureSecret` and the
	/// consumer's boot script both compose.
	pub fn password(&self) -> Variable {
		Variable::param(self.password_variable()).with_sensitive(true)
	}

	/// The tofu variable the master password arrives as, ie `db_password`. The
	/// instance is created with it and `EnsureSecret` supplies it, so both ends
	/// read the key from here.
	pub fn password_variable(&self) -> SmolStr {
		format!("{}_password", self.label).into()
	}

	/// The label suffix every security group takes, so the box's `mail--sg` and
	/// this instance's `db--sg` compose identically on both sides of an
	/// admission.
	pub const SECURITY_GROUP: &'static str = "sg";

	/// The terraform ident of this instance's own security group, the one a
	/// consumer's ingress rule points at.
	pub fn security_group_ident(&self, stack: &ResolvedStack) -> terra::Ident {
		stack.resource_ident(format!(
			"{}--{}",
			self.label,
			Self::SECURITY_GROUP
		))
	}

	/// An interpolated reference to the security group's id, ie what a
	/// consumer's ingress rule targets.
	pub fn security_group_id(&self, stack: &ResolvedStack) -> String {
		format!(
			"${{aws_security_group.{}.id}}",
			self.security_group_ident(stack).label()
		)
	}

	/// The [`SecretRef`] the master password lives at, ie
	/// `/beetmash/prod/db-password`: the declaring block grants it,
	/// `EnsureSecret` creates it and the consumer's boot script reads it, and
	/// one composition keeps the three from drifting.
	pub fn secret(&self) -> SecretRef {
		SecretRef::new(format!("{}-password", self.label))
	}

	/// The SSM parameter the master password lives in, ie
	/// `/beetmash/prod/db-password`: the [`secret`](Self::secret) composition
	/// unless [`secret`](Self::with_secret) overrides it, for a password managed
	/// outside this stack entirely.
	pub fn secret_name(&self, stack: &ResolvedStack) -> String {
		self.secret
			.clone()
			.map(Into::into)
			.unwrap_or_else(|| self.secret().name(stack))
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

	/// Reject a declaration that cannot apply, at config time: a name the
	/// engine will refuse after twenty minutes of provisioning. (A database
	/// nothing may reach fails at render, where its [`DatabaseConsumers`]
	/// resolve.)
	pub fn validate(&self) -> Result {
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
	fn label(&self) -> &SmolStr { &self.label }

	/// What a process running beside this instance does with it: read the
	/// master password out of the parameter it was put in. The connection
	/// itself needs no cloud permission at all, being a tcp session inside the
	/// vpc that the security group either admits or does not.
	fn grants(&self, stack: &ResolvedStack) -> Vec<AccessGrant> {
		vec![AccessGrant::read(
			Self::ACCESS_KIND,
			self.secret_name(stack),
		)]
	}

	/// The sensitive password variable the apply resolves.
	fn variables(&self) -> Vec<Variable> { vec![self.password()] }
}

/// The [`DeployRender`] render system, registered by [`InfraPlugin`] beside
/// the type registration.
impl RdsPostgresBlock {
	/// Render the instance and its secondaries into the config, resolving the
	/// [`VpcRef`] relation to the network it lives in. A database whose
	/// [`DatabaseConsumers`] are missing or empty is a typo, not a
	/// configuration, so it fails here rather than provisioning twenty minutes
	/// of unreachable instance.
	pub(crate) fn render(
		mut scopes: AncestorQuery<&mut RenderScope>,
		blocks: Query<(
			Entity,
			&RdsPostgresBlock,
			Option<&VpcRef>,
			Option<&DatabaseConsumers>,
		)>,
		vpcs: Query<&VpcBlock>,
	) {
		for (entity, block, vpc_ref, consumers) in blocks.iter() {
			// skip blocks outside every rendering scope before anything errors
			if scopes.get_entity(entity).is_err() {
				continue;
			}
			let vpc = crate::types::related(
				&scopes,
				entity,
				&vpcs,
				vpc_ref.map(|vpc_ref| vpc_ref.0),
				"VpcRef",
				block.label(),
			);
			let Ok(mut scope) = scopes.get_mut(entity) else {
				continue;
			};
			if consumers.is_none_or(|consumers| consumers.is_empty()) {
				scope.error(bevyhow!(
					"database '{}' admits no security group, so nothing can \
					 reach it: relate its consumers with `DatabaseRef`",
					block.label()
				));
				continue;
			}
			match vpc {
				Err(err) => scope.error(err),
				Ok(vpc) => {
					let (stack, _deployment, config) = scope.ctx();
					if let Err(err) = block.emit(stack, config, vpc) {
						scope.error(bevyhow!(
							"RdsPostgresBlock '{}': {err}",
							block.label()
						));
					}
				}
			}
		}
	}

	/// Emit this instance's resources: the password variable, the security
	/// group, then the subnet group and the instance. (The per-consumer ingress
	/// rules belong to the consumers, each of which emits its own admission
	/// through its [`DatabaseRef`] target.)
	fn emit(
		&self,
		stack: &ResolvedStack,
		config: &mut terra::Config,
		vpc: &VpcBlock,
	) -> Result {
		self.validate()?;
		let password = self.password();
		config.ensure_variable(
			password.key().as_str(),
			password.tf_declaration(),
		);
		let group = self.emit_security_group(stack, config, vpc)?;
		self.emit_instance(stack, config, vpc, &group)?;
		Ok(())
	}
}

impl RdsPostgresBlock {
	/// The instance's security group. The ingress rules live with the consumers
	/// they admit, and there is no egress rule at all, which is not an omission:
	/// declaring a group with rules of its own replaces the default
	/// allow-everything egress, and a database has nothing to call out to.
	fn emit_security_group(
		&self,
		stack: &ResolvedStack,
		config: &mut terra::Config,
		vpc: &VpcBlock,
	) -> Result<ResourceDef<AwsSecurityGroupDetails>> {
		let group = ResourceDef::new_primary(
			self.security_group_ident(stack),
			AwsSecurityGroupDetails {
				description: Some(
					format!("Postgres access for {}", self.label).into(),
				),
				vpc_id: Some(vpc.id(stack).into()),
				tags: Some(self.tags(stack, Self::SECURITY_GROUP)),
				..default()
			},
		);
		config.add_resource(&group)?;
		group.xok()
	}

	/// The subnet group spanning the vpc's private subnets, and the instance
	/// itself.
	fn emit_instance(
		&self,
		stack: &ResolvedStack,
		config: &mut terra::Config,
		vpc: &VpcBlock,
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
				subnet_ids: vpc.subnet_ids(stack, SubnetTier::Private),
				tags: Some(self.tags(stack, Self::SUBNET_GROUP)),
				..default()
			},
		);
		let identifier = self.rds_name(stack, Self::INSTANCE);
		let instance = ResourceDef::new_secondary(
			self.ident(stack),
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
				password: Some(self.password().tf_var_ref().into()),
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
					value: instance.field_ref("endpoint").into(),
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

/// The database a block consumes: the source half of the [`DatabaseConsumers`]
/// relationship, on the consumer's entity, targeting a declaration carrying an
/// [`RdsPostgresBlock`]. Authored in markup as `{DatabaseRef($db)}` beside an
/// `<RdsPostgresBlock bx:ref="db"/>`.
///
/// The counterpart of [`VpcRef`]: the consumer's render system reads the block
/// off the target and composes its connection references and its own ingress
/// admission through it, so a consumer's connection string and the instance it
/// points at cannot drift.
#[derive(Debug, Clone, PartialEq, Eq, Reflect, Component)]
#[reflect(Component)]
#[relationship(relationship_target = DatabaseConsumers)]
pub struct DatabaseRef(#[entities] pub Entity);

/// Every consumer admitted to a database: the target half of the
/// [`DatabaseRef`] relationship, on the database's declaration entity. A
/// database whose consumers are missing or empty fails the render, since a
/// database nothing may reach is a typo rather than a configuration.
#[derive(Debug, Default, Reflect, Component)]
#[reflect(Component)]
#[relationship_target(relationship = DatabaseRef)]
pub struct DatabaseConsumers(Vec<Entity>);

impl DatabaseConsumers {
	/// Whether no consumer relates to this database.
	pub fn is_empty(&self) -> bool { self.0.is_empty() }
}

#[cfg(test)]
mod tests {
	use super::*;
	use serde_json::Value;

	/// The mail stack's database as the plan declares it.
	fn database() -> RdsPostgresBlock {
		RdsPostgresBlock::new("db").with_database("mail")
	}

	/// The Sydney stack every test renders against.
	fn sydney_stack() -> Stack {
		Stack::new("beet_infra").with_region(aws::region::AP_SOUTHEAST_2)
	}

	/// The config `block` emits against a Sydney stack, related to its `net`
	/// vpc and admitting one bare consumer.
	fn build_config(
		block: &RdsPostgresBlock,
	) -> (ResolvedStack, terra::Config) {
		let block = block.clone();
		let (scope, _dir) =
			RenderScope::test_render_stack(sydney_stack(), |parent| {
				let vpc = parent.spawn(VpcBlock::new("net")).id();
				let db = parent.spawn((block, VpcRef(vpc))).id();
				parent.spawn(DatabaseRef(db));
			});
		let (stack, _deployment, config) = scope.finish().unwrap();
		(stack, config)
	}

	/// The sole `aws_db_instance` the config carries.
	fn instance(config: &terra::Config) -> Value {
		config.to_json().into_json()["resource"]["aws_db_instance"]
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
		config.to_json().into_json()["resource"]["aws_db_subnet_group"]
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
		let vpc = VpcBlock::new("net");
		config.to_json().into_json()["resource"]["aws_db_subnet_group"]
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

	/// The database emits its group and no admission of its own: every ingress
	/// rule belongs to the consumer it admits (each emits one through its
	/// [`DatabaseRef`] target), and there is no egress rule at all.
	#[beet_core::test]
	fn the_database_admits_nothing_by_itself() {
		let (_stack, config) = build_config(&database());
		config.to_json().into_json()["resource"]["aws_security_group_rule"]
			.is_null()
			.xpect_true();
		config
			.to_json_string()
			.unwrap()
			.as_str()
			.xnot()
			.xpect_contains("\"egress\"");
	}

	/// A database nothing may reach is a typo, not a configuration, so it fails
	/// the render rather than provisioning twenty minutes of unreachable
	/// instance.
	#[beet_core::test]
	fn a_database_with_no_consumer_is_an_error() {
		let (scope, _dir) =
			RenderScope::test_render_stack(sydney_stack(), |parent| {
				let vpc = parent.spawn(VpcBlock::new("net")).id();
				parent.spawn((database(), VpcRef(vpc)));
			});
		scope
			.finish()
			.unwrap_err()
			.to_string()
			.xpect_contains("admits no security group")
			.xpect_contains("`DatabaseRef`");
		RdsPostgresBlock::new("db")
			.with_database("Mail-DB")
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
		let (scope, _dir) =
			RenderScope::test_render_stack(sydney_stack(), |parent| {
				let vpc = parent.spawn(VpcBlock::new("net")).id();
				let db = parent.spawn((database(), VpcRef(vpc))).id();
				parent.spawn(DatabaseRef(db));
			});
		// ..and the deploy knows to resolve it, so `apply` passes a `-var`
		scope.variables().len().xpect_eq(1);
		let (_stack, _deployment, config) = scope.finish().unwrap();
		let json = config.to_json().into_json();
		instance(&config)["password"]
			.as_str()
			.unwrap()
			.xpect_eq("${var.db_password}");
		json["variable"]["db_password"]["sensitive"]
			.as_bool()
			.unwrap()
			.xpect_true();
	}

	/// The one permission a process beside this instance needs, which is to
	/// read the password out of the parameter it was put in. The connection
	/// itself is a tcp session the security group either admits or does not.
	#[beet_core::test]
	fn grants_read_on_its_own_secret() {
		let (scope, _dir) =
			RenderScope::test_render_stack(sydney_stack(), |parent| {
				let vpc = parent.spawn(VpcBlock::new("net")).id();
				let db = parent.spawn((database(), VpcRef(vpc))).id();
				parent.spawn(DatabaseRef(db));
			});
		scope.access().to_vec().xpect_eq(vec![AccessGrant::read(
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
		let (scope, _dir) =
			RenderScope::test_render_stack(sydney_stack(), |parent| {
				let vpc = parent.spawn(VpcBlock::new("net")).id();
				let db = parent.spawn((database(), VpcRef(vpc))).id();
				parent.spawn(DatabaseRef(db));
			});
		scope.project().unwrap().validate().await.unwrap();
	}

	/// The block's compositions are how a consumer builds its connection string
	/// and its admission, so the addresses they build must be the instance and
	/// group actually emitted.
	#[beet_core::test]
	fn compositions_point_at_the_emitted_instance() {
		let (stack, config) = build_config(&database());
		let label = database().ident(&stack).label().to_string();
		config.to_json().into_json()["resource"]["aws_db_instance"]
			.as_object()
			.unwrap()
			.contains_key(label.as_str())
			.xpect_true();
		database()
			.host(&stack)
			.as_str()
			.xpect_eq(format!("${{aws_db_instance.{label}.address}}"));
		// ..and the group a consumer's ingress rule targets is the one emitted
		config.to_json().into_json()["resource"]["aws_security_group"]
			.as_object()
			.unwrap()
			.contains_key(database().security_group_ident(&stack).label())
			.xpect_true();
	}
}
