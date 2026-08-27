use crate::bindings::*;
use crate::prelude::*;
use beet_core::prelude::*;
use serde_json::json;

/// Container image base to use for the Fargate task.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub enum ContainerImage {
	/// Scratch image - most lightweight, requires static binary.
	Scratch,
	/// Alpine Linux - lightweight with shell and basic utilities, uses musl libc.
	Alpine,
	/// Debian slim - small Debian-based image with glibc.
	#[default]
	DebianSlim,
}

impl ContainerImage {
	/// Get the base image name for Dockerfile FROM instruction.
	pub fn base_image(&self) -> &str {
		match self {
			Self::Scratch => "scratch",
			Self::Alpine => "alpine:latest",
			Self::DebianSlim => "debian:bookworm-slim",
		}
	}

	/// Whether this image needs a shell entrypoint.
	pub fn needs_entrypoint(&self) -> bool { matches!(self, Self::Scratch) }
}

/// Opinionated terraform configuration for an AWS Fargate service fronted by a
/// single Network Load Balancer:
/// - ECS cluster, task definition, and service (autoscaled on CPU)
/// - ECR repository for container images
/// - VPC with public subnets across two availability zones
/// - One NLB serving HTTP on 80, raw-TCP ssh on 22, and HTTPS on 443 (TLS
///   terminated with an ACM certificate)
/// - IAM roles for task execution and runtime, a CloudWatch log group
/// - ACM certificate (DNS-validated) and the public DNS record, when [`dns`] is
///   set
///
/// One NLB carries every transport: it operates at layer 4 and uses no security
/// group, so the task security group accepts the container ports directly. SSH
/// is raw TCP (the NLB cannot terminate it), while 80 and 443 forward to the
/// same HTTP target group (443 after terminating TLS).
///
/// [`dns`]: Self::dns
#[derive(Debug, Clone, Get, SetWith, Serialize, Deserialize, Component)]
#[component(immutable, on_add = ErasedBlock::on_add::<FargateBlock>)]
pub struct FargateBlock {
	/// Label used as a prefix for all terraform resources.
	/// Also used as the artifact name and ECR repository.
	label: SmolStr,
	/// Tofu variables to be inserted as environment variables
	/// in the Fargate task.
	#[serde(default)]
	env_vars: Vec<Variable>,
	/// The deployed binary's [`BootstrapConfig`], split at render time by
	/// [`BootstrapConfig::split_channels`]: boot selection (the store, the
	/// `--server` set) rides the container `CMD` that [`BuildDockerImage`] writes,
	/// ambient service config (the stage, the service access, the analytics table)
	/// rides the task-definition env. The platform bindings the block itself owns
	/// (the bind host, the container ports, the deploy identity) are merged in at
	/// render time, so a caller never sets them.
	#[serde(default)]
	bootstrap: BootstrapConfig,
	/// Secret env injected literally into the task definition, eg
	/// `BEET_SSH_HOST_KEY` so every task shares one stable ssh fingerprint.
	///
	/// Its own list rather than a [`BootstrapConfig`] field, so the secret channel
	/// is visible in the type and no renderer can ever put a secret on an argv
	/// line, in a `CMD` array or in a systemd `ExecStart`. Baking a secret into
	/// the task definition is a pre-existing exposure, noted, not widened.
	#[serde(default)]
	#[set_with(skip)]
	secret_env: Vec<(SmolStr, SmolStr)>,
	/// DNS + HTTPS configuration, one entry per hostname the NLB answers. When
	/// non-empty, a single ACM certificate is provisioned covering every
	/// authority (the first is the cert's primary domain, the rest are subject
	/// alternative names), DNS-validated through these providers; a 443 TLS
	/// listener is added; and a record per authority points at the NLB. Add one
	/// with [`with_dns`](Self::with_dns).
	#[serde(default)]
	#[set_with(skip)]
	dns: Vec<DnsProvider>,
	/// Port the application listens on inside the container.
	container_port: u16,
	/// Port the SSH server listens on inside the container, forwarded by the
	/// NLB's port-22 listener. Defaults to 22 so `ssh <host>` needs no `-p`; the
	/// container runs as root and so can bind the privileged port.
	ssh_container_port: u16,
	/// Whether to provision ssh (the NLB ssh listener + target group, task
	/// security-group ssh ingress, and the `BEET_SSH_PORT` env var). When
	/// `false` the deployment is http(s)-only.
	allow_ssh: bool,
	/// Task CPU units (256, 512, 1024, 2048, 4096).
	cpu: u16,
	/// Task memory in MB (512, 1024, 2048, 4096, 8192, etc).
	memory: u16,
	/// Desired number of tasks to run, kept within `[min_count, max_count]`.
	desired_count: u32,
	/// Minimum number of tasks the autoscaling target may scale down to.
	min_count: u32,
	/// Maximum number of tasks the autoscaling target may scale up to.
	max_count: u32,
	/// Target average CPU utilization (percent) for target tracking autoscaling.
	cpu_target_percent: f64,
	/// Health check path for the HTTP target group.
	health_check_path: SmolStr,
	/// Container image base.
	container_image: ContainerImage,
	/// The deploy layer for the image registry
	/// ([`Config::STORAGE_LAYER`](terra::Config::STORAGE_LAYER) by default): the
	/// deploy pushes the image into the registry before the task definition
	/// names that image, so it converges ahead of the push.
	registry_layer: SmolStr,
}

impl Default for FargateBlock {
	fn default() -> Self {
		Self {
			label: "main-fargate".into(),
			registry_layer: terra::Config::STORAGE_LAYER.into(),
			dns: Vec::new(),
			container_port: beet_net::prelude::DEFAULT_HTTP_PORT,
			ssh_container_port: 22,
			allow_ssh: false,
			cpu: 256,
			memory: 512,
			desired_count: 1,
			min_count: 1,
			max_count: 4,
			cpu_target_percent: 50.0,
			health_check_path: "/health".into(),
			container_image: ContainerImage::default(),
			env_vars: Vec::new(),
			bootstrap: default(),
			secret_env: Vec::new(),
		}
	}
}

impl FargateBlock {
	/// Build a prefixed label for terraform resources.
	pub fn build_label(&self, suffix: &str) -> String {
		format!("{}--{}", self.label, suffix)
	}

	/// Add a hostname the NLB answers (the first added is the ACM cert's primary
	/// domain, the rest are subject alternative names). See [`dns`](Self::dns).
	pub fn with_dns(mut self, dns: DnsProvider) -> Self {
		self.dns.push(dns);
		self
	}

	/// Add a secret environment variable to the task (see
	/// [`secret_env`](Self::secret_env)).
	pub fn with_secret_env(
		mut self,
		key: impl Into<SmolStr>,
		value: impl Into<SmolStr>,
	) -> Self {
		self.secret_env.push((key.into(), value.into()));
		self
	}

	/// The deployed binary's argv config: the block's own [`bootstrap`](Self::bootstrap)
	/// reduced to its argv channel, which [`BuildDockerImage`] renders into the
	/// image `CMD`.
	pub fn cmd_bootstrap(&self) -> BootstrapConfig {
		self.bootstrap.clone().split_channels().0
	}

	/// Generate a shortened name for AWS resources with length limits (eg LB +
	/// target-group names, which are account-region-global and max 32 chars).
	/// Includes the stack prefix in the length calculation and falls back to a
	/// hash when over 32 characters.
	fn short_name(&self, stack: &ResolvedStack, suffix: &str) -> SmolStr {
		use std::collections::hash_map::DefaultHasher;
		use std::hash::Hash;
		use std::hash::Hasher;

		let full = format!(
			"{}--{}--{}--{}",
			stack.app_name(),
			stack.stage(),
			self.label,
			suffix
		);
		if full.len() <= 32 {
			full.into()
		} else {
			let mut hasher = DefaultHasher::new();
			full.hash(&mut hasher);
			let hash = format!("{:x}", hasher.finish());
			if suffix.len() + 9 <= 32 {
				format!("{}-{}", &hash[..8], suffix).into()
			} else {
				hash[..31].to_string().into()
			}
		}
	}

	/// Get the container image URI for the task definition.
	fn container_image_uri(
		&self,
		deployment: &Deployment,
		ecr_repo_url_ref: &str,
	) -> SmolStr {
		format!("{}:{}", ecr_repo_url_ref, deployment.deploy_id()).into()
	}
}

impl Block for FargateBlock {
	fn artifact_label(&self) -> Option<&str> { Some(&self.label) }
	fn variables(&self) -> Vec<Variable> { self.env_vars.clone() }

	fn apply_to_config(
		&self,
		_entity: &EntityRef,
		stack: &ResolvedStack,
		deployment: &Deployment,
		_access: &AccessGrants,
		config: &mut terra::Config,
	) -> Result {
		let region = stack.region();
		let app_name = stack.app_name();
		let stage = stack.stage();
		let deploy_id = deployment.deploy_id();
		let deploy_timestamp = deployment.deploy_timestamp();

		// declare terraform variables for env_vars
		for variable in &self.env_vars {
			config.ensure_variable(
				variable.key().as_str(),
				variable.tf_declaration(),
			);
		}

		// CloudWatch log group for container logs
		let log_group = terra::ResourceDef::new_secondary(
			stack.resource_ident(self.build_label("logs")),
			AwsCloudwatchLogGroupDetails {
				name: Some(format!("/ecs/{}/{}", app_name, stage).into()),
				retention_in_days: Some(30),
				..default()
			},
		);

		// ECR repository for container images
		let ecr_repo = terra::ResourceDef::new_primary(
			stack.resource_ident(self.build_label("ecr")),
			AwsEcrRepositoryDetails {
				force_delete: Some(true),
				..default()
			},
		);
		let ecr_url_ref = ecr_repo.field_ref("repository_url");

		// VPC for Fargate tasks
		let vpc = terra::ResourceDef::new_secondary(
			stack.resource_ident(self.build_label("vpc")),
			AwsVpcDetails {
				cidr_block: Some("10.0.0.0/16".into()),
				enable_dns_hostnames: Some(true),
				enable_dns_support: Some(true),
				tags: Some(self.name_tags(stack, "vpc")),
				..default()
			},
		);

		// Internet Gateway
		let igw = terra::ResourceDef::new_secondary(
			stack.resource_ident(self.build_label("igw")),
			AwsInternetGatewayDetails {
				vpc_id: Some(vpc.field_ref("id").into()),
				tags: Some(self.name_tags(stack, "igw")),
				..default()
			},
		);

		// Public subnets in two AZs
		let subnet_a = terra::ResourceDef::new_secondary(
			stack.resource_ident(self.build_label("subnet-a")),
			AwsSubnetDetails {
				vpc_id: vpc.field_ref("id").into(),
				cidr_block: Some("10.0.1.0/24".into()),
				availability_zone: Some(format!("{}a", region).into()),
				map_public_ip_on_launch: Some(true),
				tags: Some(self.name_tags(stack, "subnet-a")),
				..default()
			},
		);
		let subnet_b = terra::ResourceDef::new_secondary(
			stack.resource_ident(self.build_label("subnet-b")),
			AwsSubnetDetails {
				vpc_id: vpc.field_ref("id").into(),
				cidr_block: Some("10.0.2.0/24".into()),
				availability_zone: Some(format!("{}b", region).into()),
				map_public_ip_on_launch: Some(true),
				tags: Some(self.name_tags(stack, "subnet-b")),
				..default()
			},
		);

		// Route table with a default route to the internet gateway
		let route_table = terra::ResourceDef::new_secondary(
			stack.resource_ident(self.build_label("route-table")),
			AwsRouteTableDetails {
				vpc_id: vpc.field_ref("id").into(),
				tags: Some(self.name_tags(stack, "route-table")),
				..default()
			},
		);
		let igw_route = terra::ResourceDef::new_secondary(
			stack.resource_ident(self.build_label("igw-route")),
			AwsRouteDetails {
				route_table_id: route_table.field_ref("id").into(),
				destination_cidr_block: Some("0.0.0.0/0".into()),
				gateway_id: Some(igw.field_ref("id").into()),
				..default()
			},
		);
		let rt_assoc_a = terra::ResourceDef::new_secondary(
			stack.resource_ident(self.build_label("rt-assoc-a")),
			AwsRouteTableAssociationDetails {
				subnet_id: Some(subnet_a.field_ref("id").into()),
				route_table_id: route_table.field_ref("id").into(),
				..default()
			},
		);
		let rt_assoc_b = terra::ResourceDef::new_secondary(
			stack.resource_ident(self.build_label("rt-assoc-b")),
			AwsRouteTableAssociationDetails {
				subnet_id: Some(subnet_b.field_ref("id").into()),
				route_table_id: route_table.field_ref("id").into(),
				..default()
			},
		);

		// Task security group. The NLB is layer 4 with no security group and
		// preserves client IPs, so the container ports accept traffic directly
		// from anywhere rather than from a load-balancer security group.
		let task_sg = terra::ResourceDef::new_primary(
			stack.resource_ident(self.build_label("task-sg")),
			AwsSecurityGroupDetails {
				description: Some("Security group for ECS tasks".into()),
				vpc_id: Some(vpc.field_ref("id").into()),
				tags: Some(self.name_tags(stack, "task-sg")),
				..default()
			},
		);
		let task_sg_http_ingress = terra::ResourceDef::new_secondary(
			stack.resource_ident(self.build_label("task-sg-http-in")),
			AwsSecurityGroupRuleDetails {
				security_group_id: task_sg.field_ref("id").into(),
				r#type: "ingress".into(),
				from_port: self.container_port.into(),
				to_port: self.container_port.into(),
				protocol: "tcp".into(),
				cidr_blocks: Some(vec!["0.0.0.0/0".into()]),
				..default()
			},
		);
		let task_sg_ssh_ingress = self.allow_ssh.then(|| {
			terra::ResourceDef::new_secondary(
				stack.resource_ident(self.build_label("task-sg-ssh-in")),
				AwsSecurityGroupRuleDetails {
					security_group_id: task_sg.field_ref("id").into(),
					r#type: "ingress".into(),
					from_port: self.ssh_container_port.into(),
					to_port: self.ssh_container_port.into(),
					protocol: "tcp".into(),
					cidr_blocks: Some(vec!["0.0.0.0/0".into()]),
					..default()
				},
			)
		});
		let task_sg_egress = terra::ResourceDef::new_secondary(
			stack.resource_ident(self.build_label("task-sg-out")),
			AwsSecurityGroupRuleDetails {
				security_group_id: task_sg.field_ref("id").into(),
				r#type: "egress".into(),
				from_port: 0,
				to_port: 0,
				protocol: "-1".into(),
				cidr_blocks: Some(vec!["0.0.0.0/0".into()]),
				..default()
			},
		);

		// The one Network Load Balancer. Cross-zone balancing lets either AZ's
		// node reach a single task in the other AZ.
		let nlb = terra::ResourceDef::new_secondary(
			stack.resource_ident(self.build_label("lb")),
			AwsLbDetails {
				name: Some(self.short_name(stack, "lb")),
				load_balancer_type: Some("network".into()),
				enable_cross_zone_load_balancing: Some(true),
				subnets: Some(vec![
					subnet_a.field_ref("id").into(),
					subnet_b.field_ref("id").into(),
				]),
				tags: Some(self.name_tags(stack, "lb")),
				..default()
			},
		);

		// HTTP target group (TCP at layer 4, HTTP `/health` check). 80 and 443
		// both forward here. `preserve_client_ip` is off by default for ip
		// targets, which would make every peer addr the NLB's private ip; the
		// real client ip is needed for the analytics geoip country.
		let http_tg = terra::ResourceDef::new_secondary(
			stack.resource_ident(self.build_label("tg")),
			AwsLbTargetGroupDetails {
				name: Some(self.short_name(stack, "tg")),
				port: Some(self.container_port.into()),
				protocol: Some("TCP".into()),
				target_type: Some("ip".into()),
				preserve_client_ip: Some("true".into()),
				vpc_id: Some(vpc.field_ref("id").into()),
				health_check: Some(vec![
					AwsLbTargetGroupResourceBlockTypeHealthCheck {
						enabled: Some(true),
						protocol: Some("HTTP".into()),
						path: Some(self.health_check_path.clone()),
						matcher: Some("200".into()),
						interval: Some(30),
						// NLB HTTP checks require equal healthy/unhealthy counts.
						healthy_threshold: Some(3),
						unhealthy_threshold: Some(3),
						..default()
					},
				]),
				..default()
			},
		);

		// HTTP listener on 80 -> http target group.
		let http_listener = terra::ResourceDef::new_secondary(
			stack.resource_ident(self.build_label("listener-http")),
			AwsLbListenerDetails {
				load_balancer_arn: nlb.field_ref("arn").into(),
				port: Some(80),
				protocol: Some("TCP".into()),
				default_action: Some(vec![forward_to(&http_tg)]),
				..default()
			},
		);

		// SSH target group + listener on 22 (raw TCP), only when `allow_ssh`.
		let ssh_infra = self.allow_ssh.then(|| {
			let ssh_tg = terra::ResourceDef::new_secondary(
				stack.resource_ident(self.build_label("ssh-tg")),
				AwsLbTargetGroupDetails {
					name: Some(self.short_name(stack, "ssh-tg")),
					port: Some(self.ssh_container_port.into()),
					protocol: Some("TCP".into()),
					target_type: Some("ip".into()),
					preserve_client_ip: Some("true".into()),
					vpc_id: Some(vpc.field_ref("id").into()),
					health_check: Some(vec![
						AwsLbTargetGroupResourceBlockTypeHealthCheck {
							enabled: Some(true),
							protocol: Some("TCP".into()),
							interval: Some(30),
							healthy_threshold: Some(3),
							unhealthy_threshold: Some(3),
							..default()
						},
					]),
					..default()
				},
			);
			let ssh_listener = terra::ResourceDef::new_secondary(
				stack.resource_ident(self.build_label("listener-ssh")),
				AwsLbListenerDetails {
					load_balancer_arn: nlb.field_ref("arn").into(),
					port: Some(self.ssh_container_port.into()),
					protocol: Some("TCP".into()),
					default_action: Some(vec![forward_to(&ssh_tg)]),
					..default()
				},
			);
			(ssh_tg, ssh_listener)
		});

		// HTTPS: a SAN ACM cert (DNS-validated) + a 443 TLS listener, only when a
		// dns authority is configured.
		let https_infra = (!self.dns.is_empty())
			.then(|| self.emit_https(stack, config, &nlb, &http_tg))
			.transpose()?;

		// IAM execution role (ECS pulls images + writes logs). `new_primary`
		// stack-prefixes the role name (IAM role names are account-global).
		let exec_role = terra::ResourceDef::new_primary(
			stack.resource_ident(self.build_label("exec-role")),
			AwsIamRoleDetails {
				assume_role_policy: ecs_assume_role_policy(),
				..default()
			},
		);
		let exec_policy = terra::ResourceDef::new_secondary(
			stack.resource_ident(self.build_label("exec-policy")),
			AwsIamRolePolicyAttachmentDetails {
				policy_arn: "arn:aws:iam::aws:policy/service-role/AmazonECSTaskExecutionRolePolicy"
					.into(),
				role: exec_role.field_ref("name").into(),
				..default()
			},
		);

		// IAM task role (runtime S3 + DynamoDB access, ie the app bucket and the
		// analytics table), stack-prefixed for the same reason.
		let task_role = terra::ResourceDef::new_primary(
			stack.resource_ident(self.build_label("task-role")),
			AwsIamRoleDetails {
				assume_role_policy: ecs_assume_role_policy(),
				..default()
			},
		);
		let task_s3_policy = terra::ResourceDef::new_secondary(
			stack.resource_ident(self.build_label("task-s3-policy")),
			AwsIamRolePolicyAttachmentDetails {
				policy_arn: "arn:aws:iam::aws:policy/AmazonS3FullAccess".into(),
				role: task_role.field_ref("name").into(),
				..default()
			},
		);
		let task_dynamo_policy = terra::ResourceDef::new_secondary(
			stack.resource_ident(self.build_label("task-dynamo-policy")),
			AwsIamRolePolicyAttachmentDetails {
				policy_arn: "arn:aws:iam::aws:policy/AmazonDynamoDBFullAccess"
					.into(),
				role: task_role.field_ref("name").into(),
				..default()
			},
		);

		// ECS cluster
		let cluster = terra::ResourceDef::new_primary(
			stack.resource_ident(self.build_label("cluster")),
			AwsEcsClusterDetails {
				name: self.build_label("cluster").into(),
				..default()
			},
		);

		// Container environment variables. The `BEET_*` set is rendered from the
		// block's `BootstrapConfig` with the platform bindings merged in, so the
		// names the deploy writes are by construction the names the runtime reads.
		let runtime = BootstrapConfig {
			host: Some(core::net::Ipv4Addr::UNSPECIFIED.into()),
			http_port: Some(self.container_port),
			ssh_port: self.allow_ssh.then_some(self.ssh_container_port),
			// the deployed stage, so the running process reports (and names cloud
			// resources for) the stage it is actually deployed to.
			stage: stage.clone(),
			deploy_id: Some(deploy_id.to_string().into()),
			deploy_timestamp: Some(deploy_timestamp.into()),
			..self.bootstrap.clone()
		};
		let mut env_vars = std::collections::BTreeMap::new();
		for (key, value) in runtime.split_channels().1.to_env() {
			env_vars.insert(key, value.to_string());
		}
		// third-party conventions the block emits directly; not beet config.
		env_vars.insert("RUST_LOG".into(), "info".into());
		env_vars.insert("AWS_REGION".into(), region.to_string());
		// env_vars as terraform variable references
		for variable in &self.env_vars {
			env_vars
				.insert(variable.key().clone(), variable.tf_var_ref().into());
		}
		// secrets, last so a deploy can override any default above.
		for (key, value) in &self.secret_env {
			env_vars.insert(key.clone(), value.to_string());
		}

		// Task definition. The http port is always mapped; the ssh port only
		// when `allow_ssh`.
		let mut port_mappings = vec![json!({
			"containerPort": self.container_port,
			"protocol": "tcp"
		})];
		if self.allow_ssh {
			port_mappings.push(json!({
				"containerPort": self.ssh_container_port,
				"protocol": "tcp"
			}));
		}
		let container_defs = json!([{
			"name": self.label.to_string(),
			"image": self.container_image_uri(deployment, &ecr_url_ref),
			"essential": true,
			"portMappings": port_mappings,
			"environment": env_vars.iter().map(|(k, v)| {
				json!({ "name": k, "value": v })
			}).collect::<Vec<_>>(),
			"logConfiguration": {
				"logDriver": "awslogs",
				"options": {
					"awslogs-group": log_group.field_ref("name"),
					"awslogs-region": region.to_string(),
					"awslogs-stream-prefix": "ecs"
				}
			}
		}])
		.to_string();
		let task_def = terra::ResourceDef::new_primary(
			stack.resource_ident(self.build_label("task-def")),
			AwsEcsTaskDefinitionDetails {
				family: self.build_label("task").into(),
				network_mode: Some("awsvpc".into()),
				requires_compatibilities: Some(vec!["FARGATE".into()]),
				cpu: Some(self.cpu.to_string().into()),
				memory: Some(self.memory.to_string().into()),
				execution_role_arn: Some(exec_role.field_ref("arn").into()),
				task_role_arn: Some(task_role.field_ref("arn").into()),
				container_definitions: container_defs.into(),
				..default()
			},
		);

		// ECS service. The http target group is always registered; the ssh
		// target group only when `allow_ssh` provisioned it.
		let desired_count =
			self.desired_count.clamp(self.min_count, self.max_count);
		let mut load_balancer =
			vec![AwsEcsServiceResourceBlockTypeLoadBalancer {
				target_group_arn: Some(http_tg.field_ref("arn").into()),
				container_name: self.label.clone(),
				container_port: self.container_port.into(),
				..default()
			}];
		if let Some((ssh_tg, _)) = &ssh_infra {
			load_balancer.push(AwsEcsServiceResourceBlockTypeLoadBalancer {
				target_group_arn: Some(ssh_tg.field_ref("arn").into()),
				container_name: self.label.clone(),
				container_port: self.ssh_container_port.into(),
				..default()
			});
		}
		let service = terra::ResourceDef::new_secondary(
			stack.resource_ident(self.build_label("service")),
			AwsEcsServiceDetails {
				name: self.build_label("service").into(),
				cluster: Some(cluster.field_ref("id").into()),
				task_definition: Some(task_def.field_ref("arn").into()),
				desired_count: Some(desired_count.into()),
				launch_type: Some("FARGATE".into()),
				// a task that cannot boot (a bad binary, a missing image) fails
				// the deployment and restores the last good task definition,
				// rather than crash-looping the new one forever. Safe only
				// because the deploy fills the stores before rolling the
				// service, see `TofuApply`: under the old ordering the first
				// tasks always failed, so this would have rolled back every
				// deploy.
				deployment_circuit_breaker: Some(vec![
					AwsEcsServiceResourceBlockTypeDeploymentCircuitBreaker {
						enable: true,
						rollback: true,
					},
				]),
				network_configuration: Some(vec![
					AwsEcsServiceResourceBlockTypeNetworkConfiguration {
						subnets: vec![
							subnet_a.field_ref("id").into(),
							subnet_b.field_ref("id").into(),
						],
						security_groups: Some(vec![
							task_sg.field_ref("id").into(),
						]),
						assign_public_ip: Some(true),
						..default()
					},
				]),
				load_balancer: Some(load_balancer),
				..default()
			},
		);

		// Application Auto Scaling target + CPU target-tracking policy.
		let scaling_resource_id = format!(
			"service/{}/{}",
			cluster.field_ref("name"),
			service.field_ref("name")
		);
		let scaling_target = terra::ResourceDef::new_secondary(
			stack.resource_ident(self.build_label("scaling-target")),
			AwsAppautoscalingTargetDetails {
				service_namespace: "ecs".into(),
				scalable_dimension: "ecs:service:DesiredCount".into(),
				resource_id: scaling_resource_id.into(),
				min_capacity: self.min_count.into(),
				max_capacity: self.max_count.into(),
				..default()
			},
		);
		let scaling_policy = terra::ResourceDef::new_secondary(
			stack.resource_ident(self.build_label("scaling-policy")),
			AwsAppautoscalingPolicyDetails {
				name: self.build_label("cpu-scaling").into(),
				policy_type: Some("TargetTrackingScaling".into()),
				service_namespace: scaling_target.field_ref("service_namespace").into(),
				scalable_dimension: scaling_target
					.field_ref("scalable_dimension")
					.into(),
				resource_id: scaling_target.field_ref("resource_id").into(),
				target_tracking_scaling_policy_configuration: Some(vec![
					AwsAppautoscalingPolicyResourceBlockTypeTargetTrackingScalingPolicyConfiguration {
						target_value: self.cpu_target_percent as i64,
						predefined_metric_specification: Some(vec![
							TargetTrackingScalingPolicyConfigurationResourceBlockTypePredefinedMetricSpecification {
								predefined_metric_type:
									"ECSServiceAverageCPUUtilization".into(),
								..default()
							},
						]),
						..default()
					},
				]),
				..default()
			},
		);

		// Add all resources. The registry converges in its layer like a bucket:
		// the deploy pushes the image into it before the task definition names
		// that image.
		config
			.add_resource(&log_group)?
			.add_layer_resource(self.registry_layer.clone(), &ecr_repo)?
			.add_resource(&vpc)?
			.add_resource(&igw)?
			.add_resource(&subnet_a)?
			.add_resource(&subnet_b)?
			.add_resource(&route_table)?
			.add_resource(&igw_route)?
			.add_resource(&rt_assoc_a)?
			.add_resource(&rt_assoc_b)?
			.add_resource(&task_sg)?
			.add_resource(&task_sg_http_ingress)?
			.add_resource(&task_sg_egress)?
			.add_resource(&nlb)?
			.add_resource(&http_tg)?
			.add_resource(&http_listener)?
			.add_resource(&exec_role)?
			.add_resource(&exec_policy)?
			.add_resource(&task_role)?
			.add_resource(&task_s3_policy)?
			.add_resource(&task_dynamo_policy)?
			.add_resource(&cluster)?
			.add_resource(&task_def)?
			.add_resource(&service)?
			.add_resource(&scaling_target)?
			.add_resource(&scaling_policy)?;

		if let Some(ssh_ingress) = &task_sg_ssh_ingress {
			config.add_resource(ssh_ingress)?;
		}
		if let Some((ssh_tg, ssh_listener)) = &ssh_infra {
			config.add_resource(ssh_tg)?.add_resource(ssh_listener)?;
		}
		if let Some(https_listener) = &https_infra {
			config.add_resource(https_listener)?;
		}

		// Outputs
		config
			.add_output("load_balancer_dns", terra::Output {
				value: json!(nlb.field_ref("dns_name")),
				description: Some("The DNS name of the load balancer".into()),
				sensitive: None,
			})?
			.add_output("cluster_name", terra::Output {
				value: json!(cluster.field_ref("name")),
				description: Some("The name of the ECS cluster".into()),
				sensitive: None,
			})?
			.add_output("service_name", terra::Output {
				value: json!(service.field_ref("name")),
				description: Some("The name of the ECS service".into()),
				sensitive: None,
			})?
			.add_output("ecr_repository_url", terra::Output {
				value: json!(ecr_repo.field_ref("repository_url")),
				description: Some("The URL of the ECR repository".into()),
				sensitive: None,
			})?;

		Ok(())
	}
}

impl FargateBlock {
	/// `Name`/`Project`/`Stage` tags for a resource, keyed by `suffix`.
	fn name_tags(
		&self,
		stack: &ResolvedStack,
		suffix: &str,
	) -> std::collections::BTreeMap<SmolStr, SmolStr> {
		[
			(SmolStr::from("Name"), self.build_label(suffix).into()),
			(SmolStr::from("Project"), stack.app_name().clone()),
			(SmolStr::from("Stage"), stack.stage().clone()),
		]
		.into_iter()
		.collect()
	}

	/// Provision a single DNS-validated ACM certificate covering every
	/// [`dns`](Self::dns) authority (first is the primary domain, rest are SANs),
	/// publish a record per authority pointing at the NLB, and return the 443 TLS
	/// listener (forwarding to `http_tg` after terminating TLS).
	fn emit_https(
		&self,
		stack: &ResolvedStack,
		config: &mut terra::Config,
		nlb: &terra::ResourceDef<AwsLbDetails>,
		http_tg: &terra::ResourceDef<AwsLbTargetGroupDetails>,
	) -> Result<terra::ResourceDef<AwsLbListenerDetails>> {
		let sans = self.dns[1..]
			.iter()
			.map(|dns| dns.authority().clone())
			.collect::<Vec<_>>();
		let cert = terra::ResourceDef::new_secondary(
			stack.resource_ident(self.build_label("cert")),
			AwsAcmCertificateDetails {
				domain_name: Some(self.dns[0].authority().clone()),
				validation_method: Some("DNS".into()),
				// Always declare the SAN set explicitly, even when empty (a
				// single-domain stack). `subject_alternative_names` is Optional +
				// Computed, so OMITTING it makes tofu keep the prior cert's SANs
				// rather than shrink the domain set, silently stranding a stale
				// multi-SAN cert when a stack drops a hostname. The provider reports
				// SANs as the extras only (excluding `domain_name`), so the declared
				// list matches the live cert and a changed set replaces it (SANs are
				// immutable; `create_before_destroy` below covers the swap).
				subject_alternative_names: Some(sans),
				..default()
			},
		);

		// One validation record per authority. `domain_validation_options` is an
		// unordered set, so each record selects its option by `domain_name` (a
		// filtered for-comprehension), trimming trailing dots the dns provider
		// rejects; the raw fqdn is kept for ACM matching.
		let dvo = cert.field("domain_validation_options");
		let mut validation_addresses = Vec::new();
		let mut validation_fqdns = Vec::new();
		for dns in &self.dns {
			let authority = dns.authority();
			let suffix = authority.replace('.', "-");
			let select = |attr: &str, trim: bool| {
				let value = format!("o.{attr}");
				let value = if trim {
					format!("trimsuffix({value}, \".\")")
				} else {
					value
				};
				format!(
					"${{[for o in {dvo} : {value} if o.domain_name == \"{authority}\"][0]}}"
				)
			};
			let address = dns.emit_cname(
				stack,
				config,
				&self.build_label(&format!("cert-validation-{suffix}")),
				&select("resource_record_name", true),
				&select("resource_record_value", true),
			)?;
			validation_addresses.push(SmolStr::from(address));
			validation_fqdns
				.push(SmolStr::from(select("resource_record_name", false)));
			// Public record: authority -> NLB.
			dns.emit(
				stack,
				config,
				&self.build_label(&format!("dns-{suffix}")),
				&nlb.field_ref("dns_name"),
			)?;
		}

		let cert_validation = terra::ResourceDef::new_secondary(
			stack.resource_ident(self.build_label("cert-validation")),
			AwsAcmCertificateValidationDetails {
				certificate_arn: cert.field_ref("arn").into(),
				validation_record_fqdns: Some(validation_fqdns),
				depends_on: Some(validation_addresses),
				..default()
			},
		);

		// 443 TLS listener using the validated cert.
		let https_listener = terra::ResourceDef::new_secondary(
			stack.resource_ident(self.build_label("listener-https")),
			AwsLbListenerDetails {
				load_balancer_arn: nlb.field_ref("arn").into(),
				port: Some(443),
				protocol: Some("TLS".into()),
				certificate_arn: Some(
					cert_validation.field_ref("certificate_arn").into(),
				),
				ssl_policy: Some("ELBSecurityPolicy-TLS13-1-2-2021-06".into()),
				default_action: Some(vec![forward_to(http_tg)]),
				..default()
			},
		);

		config.add_resource(&cert)?.add_resource(&cert_validation)?;
		// SANs are immutable, so changing the domain set replaces the cert; create
		// the replacement (and validate it) before destroying the old one, so the
		// 443 listener never references a torn-down certificate.
		config.set_lifecycle(
			"aws_acm_certificate",
			cert.ident().label(),
			json!({ "create_before_destroy": true }),
		)?;
		Ok(https_listener)
	}
}

/// A `forward` default-action targeting `tg`.
fn forward_to(
	tg: &terra::ResourceDef<AwsLbTargetGroupDetails>,
) -> AwsLbListenerResourceBlockTypeDefaultAction {
	AwsLbListenerResourceBlockTypeDefaultAction {
		r#type: "forward".into(),
		target_group_arn: Some(tg.field_ref("arn").into()),
		..default()
	}
}

/// The ECS-tasks `sts:AssumeRole` trust policy, shared by the exec + task roles.
fn ecs_assume_role_policy() -> SmolStr {
	json!({
		"Version": "2012-10-17",
		"Statement": [{
			"Action": "sts:AssumeRole",
			"Effect": "Allow",
			"Principal": { "Service": "ecs-tasks.amazonaws.com" }
		}]
	})
	.to_string()
	.into()
}

#[cfg(test)]
mod tests {
	use super::*;

	/// Build a config from the given block, returning the config, stack, and the
	/// work dir guard that keeps the local stack alive.
	fn build_config(
		block: &FargateBlock,
	) -> (terra::Config, ResolvedStack, TestWorkDir) {
		build_config_at(block, BootstrapConfig::DEFAULT_STAGE)
	}

	/// [`build_config`] against a stack deployed to `stage`.
	fn build_config_at(
		block: &FargateBlock,
		stage: &str,
	) -> (terra::Config, ResolvedStack, TestWorkDir) {
		let (stack, deployment, dir) = ResolvedStack::default_local();
		let stack = stack.with_stage(stage);
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
		(config, stack, dir)
	}

	/// Build the terraform json for the given block.
	fn build_json(block: &FargateBlock) -> String {
		build_config(block).0.to_json().to_string()
	}

	/// Assert the autoscaling target + policy are emitted regardless of ssh.
	fn xpect_autoscaling(json: &str) {
		json.xpect_contains("aws_appautoscaling_target")
			.xpect_contains("aws_appautoscaling_policy")
			.xpect_contains("ecs:service:DesiredCount")
			.xpect_contains("TargetTrackingScaling")
			.xpect_contains("ECSServiceAverageCPUUtilization")
			.xpect_contains("\"max_capacity\":7")
			.xpect_contains("\"min_capacity\":2")
			.xpect_contains("\"target_value\":65");
	}

	/// The autoscaling-tuned block shared by the tests.
	fn autoscaling_block() -> FargateBlock {
		FargateBlock::default()
			.with_min_count(2)
			.with_max_count(7)
			.with_cpu_target_percent(65.0)
	}

	/// The registry the deploy pushes its image into is the block's one storage
	/// layer entry; the service that pulls the image is not, since rolling the
	/// service onto already-published content is what the full apply *after* the
	/// push is for. The circuit breaker rides on that ordering: a task that still
	/// fails to boot now fails the deployment rather than crash-looping.
	#[beet_core::test]
	fn ecr_is_the_only_storage_target() {
		let block = FargateBlock::default();
		let (config, stack, _dir) = build_config(&block);
		config
			.layer_targets("storage")
			.unwrap()
			.to_vec()
			.xpect_eq(vec![format!(
				"aws_ecr_repository.{}",
				stack.resource_ident(block.build_label("ecr")).label()
			)]);
		config.to_json().to_string().as_str().xpect_contains(
			"\"deployment_circuit_breaker\":[{\"enable\":true,\"rollback\":true}]",
		);
	}

	#[beet_core::test]
	fn always_a_single_network_load_balancer() {
		// the lone load balancer is always an NLB, http-only or with ssh
		let json = build_json(&autoscaling_block());
		let json = json.as_str();
		json.xpect_contains("\"load_balancer_type\":\"network\"")
			.xpect_contains("\"port\":80")
			.xpect_contains("load_balancer_dns");
		// exactly one load balancer resource (no separate ssh lb anymore)
		json.matches("\"load_balancer_type\"").count().xpect_eq(1);
	}

	#[beet_core::test]
	fn allow_ssh_emits_ssh_listener_and_autoscaling() {
		let json = build_json(&autoscaling_block().with_allow_ssh(true));
		let json = json.as_str();
		xpect_autoscaling(json);
		json
			// ssh listens on 22 (no -p), forwarded to a TCP ssh target group
			.xpect_contains("\"port\":22")
			.xpect_contains("ssh-tg")
			.xpect_contains("BEET_SSH_PORT")
			// still one network load balancer, carrying ssh too
			.xpect_contains("\"load_balancer_type\":\"network\"");
		json.matches("\"load_balancer_type\"").count().xpect_eq(1);
	}

	#[beet_core::test]
	fn http_only_omits_ssh() {
		// the default is http-only: the NLB holds but no ssh infra
		let json = build_json(&autoscaling_block());
		let json = json.as_str();
		xpect_autoscaling(json);
		json.xnot()
			.xpect_contains("ssh-tg")
			.xnot()
			.xpect_contains("BEET_SSH_PORT")
			.xnot()
			.xpect_contains("\"port\":22");
	}

	#[beet_core::test]
	fn dns_emits_cert_validation_and_https_listener() {
		let json =
			build_json(&autoscaling_block().with_dns(DnsProvider::cloudflare(
				"dev.example.org",
				"zone123",
			)));
		json.as_str()
			// ACM cert, DNS validated, and the 443 TLS listener
			.xpect_contains("aws_acm_certificate")
			.xpect_contains("aws_acm_certificate_validation")
			.xpect_contains("\"validation_method\":\"DNS\"")
			// a single-domain stack still emits the SAN set explicitly (empty), so a
			// later shrink to one domain is tracked instead of keeping a stale cert.
			.xpect_contains("\"subject_alternative_names\":[]")
			.xpect_contains("\"port\":443")
			.xpect_contains("\"protocol\":\"TLS\"")
			// the public record + validation record in the cloudflare zone
			.xpect_contains("cloudflare_dns_record")
			.xpect_contains("dev.example.org")
			.xpect_contains("zone123");
	}

	#[beet_core::test]
	fn proxied_dns_emits_proxied_record() {
		let json = build_json(
			&autoscaling_block().with_dns(
				DnsProvider::cloudflare("example.org", "zone123")
					.with_proxied(true),
			),
		);
		// the record rides the edge
		json.as_str().xpect_contains("\"proxied\":true");
		// ACM validation records stay unproxied even on a proxied deploy
		json.matches("\"proxied\":false").count().xpect_eq(1);
	}

	#[beet_core::test]
	fn multiple_dns_emits_san_cert_and_record_each() {
		let json = build_json(
			&autoscaling_block()
				.with_dns(DnsProvider::cloudflare("example.org", "z"))
				.with_dns(DnsProvider::cloudflare("www.example.org", "z")),
		);
		json.as_str()
			// the second authority is a subject alternative name on one cert
			.xpect_contains("subject_alternative_names")
			.xpect_contains("www.example.org")
			.xpect_contains("example.org")
			// one cert, but a record + validation record per authority
			.xpect_contains("dns_example_org")
			.xpect_contains("dns_www_example_org");
		// exactly one ACM certificate resource for all authorities
		json.matches("\"aws_acm_certificate\"").count().xpect_eq(1);
	}

	#[beet_core::test]
	fn injects_beet_stage_env() {
		// the container must carry BEET_STAGE (set to the stack's stage) so the
		// deployed runtime reports the stage it is actually running in. Asserted
		// against a named stage, since the `dev` default renders to nothing.
		let (config, stack, _dir) =
			build_config_at(&autoscaling_block(), "prod");
		// container_definitions is a JSON string nested in the config, so the env
		// entry renders with escaped quotes (matching the BEET_SSH_PORT tests).
		config.to_json().to_string().xpect_contains(&format!(
			r#"{{\"name\":\"BEET_STAGE\",\"value\":\"{}\"}}"#,
			stack.stage()
		));
	}

	/// The task-definition env is rendered from the block's `BootstrapConfig`, so
	/// the names the deploy writes are the names the runtime parses: the platform
	/// bindings the block owns plus whatever the deploy declared.
	#[beet_core::test]
	fn renders_bootstrap_env() {
		let json = build_json(
			&FargateBlock::default()
				.with_allow_ssh(true)
				.with_container_port(9001)
				.with_bootstrap(BootstrapConfig {
					service_access: ServiceAccess::Remote,
					store: Some(
						StoreUri::parse("s3://beet--dev--app").unwrap(),
					),
					..default()
				}),
		);
		let json = json.as_str();
		json
			// the platform bindings the block merges in
			.xpect_contains(r#"\"name\":\"BEET_HOST\",\"value\":\"0.0.0.0\""#)
			.xpect_contains(r#"\"name\":\"BEET_HTTP_PORT\",\"value\":\"9001\""#)
			.xpect_contains(r#"\"name\":\"BEET_SSH_PORT\",\"value\":\"22\""#)
			// the deploy-declared service config
			.xpect_contains(
				r#"\"name\":\"BEET_SERVICE_ACCESS\",\"value\":\"remote\""#,
			)
			// entry-store selection rides argv (the image CMD), never env, and the
			// one app bucket is the only store there is: no second bucket name
			// reaches the container.
			.xnot()
			.xpect_contains("BEET_STORE")
			.xnot()
			.xpect_contains("BEET_ASSETS_BUCKET");
	}

	/// Boot selection rides the container `CMD` as a real JSON array, and the
	/// secret channel is visible in the type: a secret can only reach env.
	#[beet_core::test]
	fn splits_cmd_and_secret_channels() {
		let block = FargateBlock::default().with_bootstrap(BootstrapConfig {
			store: Some(StoreUri::parse("s3://beet--dev--app").unwrap()),
			server: Some(RunningSetFilter::new("http,ssh")),
			stage: "staging".into(),
			..default()
		});
		block.cmd_bootstrap().to_cmd_json("/app").unwrap().xpect_eq(
			r#"["/app", "--store=s3://beet--dev--app", "--server=http,ssh"]"#,
		);
		build_json(&block.with_secret_env("BEET_SSH_HOST_KEY", "abc123"))
			.as_str()
			.xpect_contains(
				r#"\"name\":\"BEET_SSH_HOST_KEY\",\"value\":\"abc123\""#,
			);
	}
}
