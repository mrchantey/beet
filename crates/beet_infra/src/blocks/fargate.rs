use crate::bindings::*;
use crate::prelude::*;
use beet_core::prelude::*;
use serde_json::json;

/// Load balancer type for Fargate deployment.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub enum LoadBalancerType {
	/// Application Load Balancer (Layer 7 HTTP/HTTPS).
	#[default]
	ApplicationLoadBalancer,
	/// Network Load Balancer (Layer 4 TCP/UDP).
	NetworkLoadBalancer,
}

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

/// Opinionated terraform configuration for AWS Fargate:
/// - ECS Cluster, Task Definition, and Service
/// - ECR repository for container images
/// - VPC with public subnets across multiple availability zones
/// - Application or Network Load Balancer with health checks
/// - IAM roles for task execution and runtime permissions
/// - CloudWatch log group for container logs
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
	/// Optional domain for HTTPS. When provided, an HTTPS listener
	/// is added to the load balancer. DNS must be configured separately.
	#[set_with(unwrap_option, into)]
	domain: Option<SmolStr>,
	/// Port the application listens on inside the container.
	container_port: u16,
	/// Port the SSH server listens on inside the container, exposed via a
	/// dedicated TCP Network Load Balancer.
	ssh_container_port: u16,
	/// Whether to provision ssh infrastructure (the NLB, ssh target group +
	/// listener, task security-group ssh ingress, and the `BEET_SSH_PORT` env
	/// var). When `false` the deployment is http-only.
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
	/// Health check path for the load balancer.
	health_check_path: SmolStr,
	/// Load balancer type (ALB or NLB).
	load_balancer_type: LoadBalancerType,
	/// Container image base.
	container_image: ContainerImage,
}

impl Default for FargateBlock {
	fn default() -> Self {
		Self {
			label: "main-fargate".into(),
			domain: None,
			container_port: beet_net::prelude::DEFAULT_SERVER_PORT,
			ssh_container_port: beet_net::prelude::DEFAULT_SSH_PORT,
			allow_ssh: false,
			cpu: 256,
			memory: 512,
			desired_count: 1,
			min_count: 1,
			max_count: 4,
			cpu_target_percent: 50.0,
			health_check_path: "/health".into(),
			load_balancer_type: LoadBalancerType::default(),
			container_image: ContainerImage::default(),
			env_vars: Vec::new(),
		}
	}
}

impl FargateBlock {
	/// Build a prefixed label for terraform resources.
	pub fn build_label(&self, suffix: &str) -> String {
		format!("{}--{}", self.label, suffix)
	}

	/// Generate a shortened name for AWS resources with length limits (e.g. ALB names).
	/// Includes stack prefix in the length calculation and uses a hash if over 32 characters.
	fn short_name(&self, stack: &Stack, suffix: &str) -> SmolStr {
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
		let result = if full.len() <= 32 {
			full.into()
		} else {
			// use hash to shorten
			let mut hasher = DefaultHasher::new();
			full.hash(&mut hasher);
			let hash = format!("{:x}", hasher.finish());
			let truncated = if suffix.len() + 9 <= 32 {
				format!("{}-{}", &hash[..8], suffix)
			} else {
				// just use hash
				hash[..31].to_string()
			};
			truncated.into()
		};
		result
	}

	/// Get the container image URI for the task definition.
	fn container_image_uri(
		&self,
		stack: &Stack,
		ecr_repo_url_ref: &str,
	) -> SmolStr {
		format!("{}:{}", ecr_repo_url_ref, stack.deploy_id()).into()
	}
}

impl Block for FargateBlock {
	fn artifact_label(&self) -> Option<&str> { Some(&self.label) }
	fn variables(&self) -> &[Variable] { &self.env_vars }

	fn apply_to_config(
		&self,
		_entity: &EntityRef,
		stack: &Stack,
		config: &mut terra::Config,
	) -> Result {
		let region = stack.aws_region();
		let app_name = stack.app_name();
		let stage = stack.stage();
		let deploy_id = stack.deploy_id();
		let deploy_timestamp = stack.deploy_timestamp();

		// declare terraform variables for env_vars
		for variable in &self.env_vars {
			config.ensure_variable(
				variable.key().as_str(),
				variable.tf_declaration(),
			);
		}

		// CloudWatch log group for container logs
		let log_group_ident = stack.resource_ident(self.build_label("logs"));
		let log_group = terra::ResourceDef::new_secondary(
			log_group_ident,
			AwsCloudwatchLogGroupDetails {
				name: Some(format!("/ecs/{}/{}", app_name, stage).into()),
				retention_in_days: Some(30),
				..default()
			},
		);

		// ECR repository for container images
		let ecr_ident = stack.resource_ident(self.build_label("ecr"));
		let ecr_repo = terra::ResourceDef::new_primary(
			ecr_ident,
			AwsEcrRepositoryDetails {
				force_delete: Some(true),
				..default()
			},
		);
		let ecr_url_ref = ecr_repo.field_ref("repository_url");

		// VPC for Fargate tasks
		let vpc_ident = stack.resource_ident(self.build_label("vpc"));
		let vpc = terra::ResourceDef::new_secondary(vpc_ident, AwsVpcDetails {
			cidr_block: Some("10.0.0.0/16".into()),
			enable_dns_hostnames: Some(true),
			enable_dns_support: Some(true),
			tags: Some(
				[
					(SmolStr::from("Name"), self.build_label("vpc").into()),
					(SmolStr::from("Project"), app_name.clone()),
					(SmolStr::from("Stage"), stage.clone()),
				]
				.into_iter()
				.collect(),
			),
			..default()
		});

		// Internet Gateway
		let igw_ident = stack.resource_ident(self.build_label("igw"));
		let igw = terra::ResourceDef::new_secondary(
			igw_ident,
			AwsInternetGatewayDetails {
				vpc_id: Some(vpc.field_ref("id").into()),
				tags: Some(
					[(SmolStr::from("Name"), self.build_label("igw").into())]
						.into_iter()
						.collect(),
				),
				..default()
			},
		);

		// Public subnets in multiple AZs
		let subnet_a_ident = stack.resource_ident(self.build_label("subnet-a"));
		let subnet_a = terra::ResourceDef::new_secondary(
			subnet_a_ident,
			AwsSubnetDetails {
				vpc_id: vpc.field_ref("id").into(),
				cidr_block: Some("10.0.1.0/24".into()),
				availability_zone: Some(format!("{}a", region).into()),
				map_public_ip_on_launch: Some(true),
				tags: Some(
					[(
						SmolStr::from("Name"),
						self.build_label("subnet-a").into(),
					)]
					.into_iter()
					.collect(),
				),
				..default()
			},
		);

		let subnet_b_ident = stack.resource_ident(self.build_label("subnet-b"));
		let subnet_b = terra::ResourceDef::new_secondary(
			subnet_b_ident,
			AwsSubnetDetails {
				vpc_id: vpc.field_ref("id").into(),
				cidr_block: Some("10.0.2.0/24".into()),
				availability_zone: Some(format!("{}b", region).into()),
				map_public_ip_on_launch: Some(true),
				tags: Some(
					[(
						SmolStr::from("Name"),
						self.build_label("subnet-b").into(),
					)]
					.into_iter()
					.collect(),
				),
				..default()
			},
		);

		// Route table with internet gateway route
		let route_table_ident =
			stack.resource_ident(self.build_label("route-table"));
		let route_table = terra::ResourceDef::new_secondary(
			route_table_ident,
			AwsRouteTableDetails {
				vpc_id: vpc.field_ref("id").into(),
				tags: Some(
					[(
						SmolStr::from("Name"),
						self.build_label("route-table").into(),
					)]
					.into_iter()
					.collect(),
				),
				..default()
			},
		);

		// Route for internet gateway
		let igw_route = terra::ResourceDef::new_secondary(
			stack.resource_ident(self.build_label("igw-route")),
			AwsRouteDetails {
				route_table_id: route_table.field_ref("id").into(),
				destination_cidr_block: Some("0.0.0.0/0".into()),
				gateway_id: Some(igw.field_ref("id").into()),
				..default()
			},
		);

		// Route table associations
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

		// Security group for ALB
		let alb_sg_ident = stack.resource_ident(self.build_label("alb-sg"));
		let alb_sg = terra::ResourceDef::new_secondary(
			alb_sg_ident,
			AwsSecurityGroupDetails {
				name: Some(self.build_label("alb-sg").into()),
				description: Some("Security group for ALB".into()),
				vpc_id: Some(vpc.field_ref("id").into()),
				tags: Some(
					[(
						SmolStr::from("Name"),
						self.build_label("alb-sg").into(),
					)]
					.into_iter()
					.collect(),
				),
				..default()
			},
		);

		// ALB security group rules - ingress HTTP
		let alb_sg_http_ingress = terra::ResourceDef::new_secondary(
			stack.resource_ident(self.build_label("alb-sg-http-in")),
			AwsSecurityGroupRuleDetails {
				security_group_id: alb_sg.field_ref("id").into(),
				r#type: "ingress".into(),
				from_port: 80,
				to_port: 80,
				protocol: "tcp".into(),
				cidr_blocks: Some(vec!["0.0.0.0/0".into()]),
				..default()
			},
		);

		// ALB security group rules - ingress HTTPS (if domain provided)
		let alb_sg_https_ingress = if self.domain.is_some() {
			Some(terra::ResourceDef::new_secondary(
				stack.resource_ident(self.build_label("alb-sg-https-in")),
				AwsSecurityGroupRuleDetails {
					security_group_id: alb_sg.field_ref("id").into(),
					r#type: "ingress".into(),
					from_port: 443,
					to_port: 443,
					protocol: "tcp".into(),
					cidr_blocks: Some(vec!["0.0.0.0/0".into()]),
					..default()
				},
			))
		} else {
			None
		};

		// ALB security group rules - egress all
		let alb_sg_egress = terra::ResourceDef::new_secondary(
			stack.resource_ident(self.build_label("alb-sg-out")),
			AwsSecurityGroupRuleDetails {
				security_group_id: alb_sg.field_ref("id").into(),
				r#type: "egress".into(),
				from_port: 0,
				to_port: 0,
				protocol: "-1".into(),
				cidr_blocks: Some(vec!["0.0.0.0/0".into()]),
				..default()
			},
		);

		// Security group for ECS tasks
		let task_sg_ident = stack.resource_ident(self.build_label("task-sg"));
		let task_sg = terra::ResourceDef::new_secondary(
			task_sg_ident,
			AwsSecurityGroupDetails {
				name: Some(self.build_label("task-sg").into()),
				description: Some("Security group for ECS tasks".into()),
				vpc_id: Some(vpc.field_ref("id").into()),
				tags: Some(
					[(
						SmolStr::from("Name"),
						self.build_label("task-sg").into(),
					)]
					.into_iter()
					.collect(),
				),
				..default()
			},
		);

		// Task security group rules - ingress from ALB
		let task_sg_ingress = terra::ResourceDef::new_secondary(
			stack.resource_ident(self.build_label("task-sg-in")),
			AwsSecurityGroupRuleDetails {
				security_group_id: task_sg.field_ref("id").into(),
				r#type: "ingress".into(),
				from_port: self.container_port.into(),
				to_port: self.container_port.into(),
				protocol: "tcp".into(),
				source_security_group_id: Some(alb_sg.field_ref("id").into()),
				..default()
			},
		);

		// Task security group rules - ingress for SSH from anywhere.
		// The NLB preserves client IPs and has no security group, so the task
		// must accept SSH traffic directly from 0.0.0.0/0.
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

		// Task security group rules - egress all
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

		// Load balancer
		let lb_type = match &self.load_balancer_type {
			LoadBalancerType::ApplicationLoadBalancer => "application",
			LoadBalancerType::NetworkLoadBalancer => "network",
		};
		let lb_ident = stack.resource_ident(self.build_label("lb"));
		let lb = terra::ResourceDef::new_secondary(lb_ident, AwsLbDetails {
			name: Some(self.short_name(stack, "lb")),
			load_balancer_type: Some(lb_type.into()),
			security_groups: Some(vec![alb_sg.field_ref("id").into()]),
			subnets: Some(vec![
				subnet_a.field_ref("id").into(),
				subnet_b.field_ref("id").into(),
			]),
			tags: Some(
				[(SmolStr::from("Name"), self.build_label("lb").into())]
					.into_iter()
					.collect(),
			),
			..default()
		});

		// Target group
		let tg_ident = stack.resource_ident(self.build_label("tg"));
		let target_group = terra::ResourceDef::new_secondary(
			tg_ident,
			AwsLbTargetGroupDetails {
				name: Some(self.build_label("tg").into()),
				port: Some(self.container_port.into()),
				protocol: Some("HTTP".into()),
				target_type: Some("ip".into()),
				vpc_id: Some(vpc.field_ref("id").into()),
				health_check: Some(vec![
					AwsLbTargetGroupResourceBlockTypeHealthCheck {
						enabled: Some(true),
						healthy_threshold: Some(2),
						interval: Some(30),
						matcher: Some("200".into()),
						path: Some(self.health_check_path.clone()),
						protocol: Some("HTTP".into()),
						timeout: Some(5),
						unhealthy_threshold: Some(2),
						..default()
					},
				]),
				..default()
			},
		);

		// HTTP listener
		let http_listener = terra::ResourceDef::new_secondary(
			stack.resource_ident(self.build_label("listener-http")),
			AwsLbListenerDetails {
				load_balancer_arn: lb.field_ref("arn").into(),
				port: Some(80),
				protocol: Some("HTTP".into()),
				default_action: Some(vec![
					AwsLbListenerResourceBlockTypeDefaultAction {
						r#type: "forward".into(),
						target_group_arn: Some(
							target_group.field_ref("arn").into(),
						),
						..default()
					},
				]),
				..default()
			},
		);

		// SSH infrastructure (NLB, TCP target group, TCP listener), only when
		// `allow_ssh` is set. NLBs operate at layer 4 and do not use security
		// groups, so the task security group accepts ssh from anywhere instead.
		let ssh_infra = self.allow_ssh.then(|| {
			// Network Load Balancer for raw TCP SSH traffic, across the same subnets.
			let ssh_lb_ident = stack.resource_ident(self.build_label("ssh-lb"));
			let ssh_lb =
				terra::ResourceDef::new_secondary(ssh_lb_ident, AwsLbDetails {
					name: Some(self.short_name(stack, "ssh-lb")),
					load_balancer_type: Some("network".into()),
					subnets: Some(vec![
						subnet_a.field_ref("id").into(),
						subnet_b.field_ref("id").into(),
					]),
					tags: Some(
						[(
							SmolStr::from("Name"),
							self.build_label("ssh-lb").into(),
						)]
						.into_iter()
						.collect(),
					),
					..default()
				});

			// TCP target group for the SSH port.
			let ssh_tg_ident = stack.resource_ident(self.build_label("ssh-tg"));
			let ssh_target_group = terra::ResourceDef::new_secondary(
				ssh_tg_ident,
				AwsLbTargetGroupDetails {
					name: Some(self.build_label("ssh-tg").into()),
					port: Some(self.ssh_container_port.into()),
					protocol: Some("TCP".into()),
					target_type: Some("ip".into()),
					vpc_id: Some(vpc.field_ref("id").into()),
					health_check: Some(vec![
						AwsLbTargetGroupResourceBlockTypeHealthCheck {
							enabled: Some(true),
							healthy_threshold: Some(2),
							interval: Some(30),
							protocol: Some("TCP".into()),
							timeout: Some(10),
							unhealthy_threshold: Some(2),
							..default()
						},
					]),
					..default()
				},
			);

			// TCP listener forwarding the SSH port to the SSH target group.
			let ssh_listener = terra::ResourceDef::new_secondary(
				stack.resource_ident(self.build_label("listener-ssh")),
				AwsLbListenerDetails {
					load_balancer_arn: ssh_lb.field_ref("arn").into(),
					port: Some(self.ssh_container_port.into()),
					protocol: Some("TCP".into()),
					default_action: Some(vec![
						AwsLbListenerResourceBlockTypeDefaultAction {
							r#type: "forward".into(),
							target_group_arn: Some(
								ssh_target_group.field_ref("arn").into(),
							),
							..default()
						},
					]),
					..default()
				},
			);

			(ssh_lb, ssh_target_group, ssh_listener)
		});

		// IAM execution role (for ECS to pull images and write logs)
		let exec_role_ident =
			stack.resource_ident(self.build_label("exec-role"));
		let exec_role = terra::ResourceDef::new_secondary(
			exec_role_ident,
			AwsIamRoleDetails {
				name: Some(self.build_label("exec-role").into()),
				assume_role_policy: json!({
					"Version": "2012-10-17",
					"Statement": [{
						"Action": "sts:AssumeRole",
						"Effect": "Allow",
						"Principal": { "Service": "ecs-tasks.amazonaws.com" }
					}]
				})
				.to_string()
				.into(),
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

		// IAM task role (for application runtime S3 access)
		let task_role_ident =
			stack.resource_ident(self.build_label("task-role"));
		let task_role = terra::ResourceDef::new_secondary(
			task_role_ident,
			AwsIamRoleDetails {
				name: Some(self.build_label("task-role").into()),
				assume_role_policy: json!({
					"Version": "2012-10-17",
					"Statement": [{
						"Action": "sts:AssumeRole",
						"Effect": "Allow",
						"Principal": { "Service": "ecs-tasks.amazonaws.com" }
					}]
				})
				.to_string()
				.into(),
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

		// ECS Cluster
		let cluster_ident = stack.resource_ident(self.build_label("cluster"));
		let cluster = terra::ResourceDef::new_primary(
			cluster_ident,
			AwsEcsClusterDetails {
				name: self.build_label("cluster").into(),
				..default()
			},
		);

		// Build environment variables map
		let mut env_vars = std::collections::BTreeMap::new();
		env_vars.insert("BEET_DEPLOY_ID".into(), deploy_id.to_string().into());
		env_vars.insert(
			"BEET_DEPLOY_TIMESTAMP".into(),
			deploy_timestamp.to_string().into(),
		);
		env_vars.insert("BEET_HOST".into(), "0.0.0.0".into());
		env_vars
			.insert("BEET_PORT".into(), self.container_port.to_string().into());
		if self.allow_ssh {
			env_vars.insert(
				"BEET_SSH_PORT".into(),
				self.ssh_container_port.to_string().into(),
			);
		}
		env_vars.insert("RUST_LOG".into(), "info".into());
		env_vars.insert("AWS_REGION".into(), region.to_string());

		// add env_vars as terraform variable references
		for variable in &self.env_vars {
			env_vars
				.insert(variable.key().clone(), variable.tf_var_ref().into());
		}

		// Task definition. The http port is always mapped; the ssh port only
		// when `allow_ssh`.
		let task_def_ident = stack.resource_ident(self.build_label("task-def"));
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
			"image": self.container_image_uri(stack, &ecr_url_ref),
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
			task_def_ident,
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

		// ECS Service
		let service_ident = stack.resource_ident(self.build_label("service"));
		// keep the desired count within the autoscaling bounds
		let desired_count =
			self.desired_count.clamp(self.min_count, self.max_count);
		// the http target group is always registered; the ssh target group only
		// when `allow_ssh` provisioned the NLB.
		let mut load_balancer = vec![AwsEcsServiceResourceBlockTypeLoadBalancer {
			target_group_arn: Some(target_group.field_ref("arn").into()),
			container_name: self.label.clone(),
			container_port: self.container_port.into(),
			..default()
		}];
		if let Some((_, ssh_target_group, _)) = &ssh_infra {
			load_balancer.push(AwsEcsServiceResourceBlockTypeLoadBalancer {
				target_group_arn: Some(ssh_target_group.field_ref("arn").into()),
				container_name: self.label.clone(),
				container_port: self.ssh_container_port.into(),
				..default()
			});
		}
		let service = terra::ResourceDef::new_secondary(
			service_ident,
			AwsEcsServiceDetails {
				name: self.build_label("service").into(),
				cluster: Some(cluster.field_ref("id").into()),
				task_definition: Some(task_def.field_ref("arn").into()),
				desired_count: Some(desired_count.into()),
				launch_type: Some("FARGATE".into()),
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

		// Application Auto Scaling target for the ECS service desired count.
		let scaling_resource_id = format!(
			"service/{}/{}",
			cluster.field_ref("name"),
			service.field_ref("name")
		);
		let scaling_target_ident =
			stack.resource_ident(self.build_label("scaling-target"));
		let scaling_target = terra::ResourceDef::new_secondary(
			scaling_target_ident,
			AwsAppautoscalingTargetDetails {
				service_namespace: "ecs".into(),
				scalable_dimension: "ecs:service:DesiredCount".into(),
				resource_id: scaling_resource_id.into(),
				min_capacity: self.min_count.into(),
				max_capacity: self.max_count.into(),
				..default()
			},
		);

		// Target tracking policy keeping average CPU at `cpu_target_percent`.
		let scaling_policy_ident =
			stack.resource_ident(self.build_label("scaling-policy"));
		let scaling_policy = terra::ResourceDef::new_secondary(
			scaling_policy_ident,
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

		// Add all resources
		config
			.add_resource(&log_group)?
			.add_resource(&ecr_repo)?
			.add_resource(&vpc)?
			.add_resource(&igw)?
			.add_resource(&subnet_a)?
			.add_resource(&subnet_b)?
			.add_resource(&route_table)?
			.add_resource(&igw_route)?
			.add_resource(&rt_assoc_a)?
			.add_resource(&rt_assoc_b)?
			.add_resource(&alb_sg)?
			.add_resource(&alb_sg_http_ingress)?
			.add_resource(&alb_sg_egress)?
			.add_resource(&task_sg)?
			.add_resource(&task_sg_ingress)?
			.add_resource(&task_sg_egress)?
			.add_resource(&lb)?
			.add_resource(&target_group)?
			.add_resource(&http_listener)?
			.add_resource(&exec_role)?
			.add_resource(&exec_policy)?
			.add_resource(&task_role)?
			.add_resource(&task_s3_policy)?
			.add_resource(&cluster)?
			.add_resource(&task_def)?
			.add_resource(&service)?
			.add_resource(&scaling_target)?
			.add_resource(&scaling_policy)?;

		if let Some(https_sg) = alb_sg_https_ingress {
			config.add_resource(&https_sg)?;
		}

		if let Some(ssh_ingress) = &task_sg_ssh_ingress {
			config.add_resource(ssh_ingress)?;
		}

		// ssh infrastructure and its output, only when `allow_ssh`
		if let Some((ssh_lb, ssh_target_group, ssh_listener)) = &ssh_infra {
			config
				.add_resource(ssh_lb)?
				.add_resource(ssh_target_group)?
				.add_resource(ssh_listener)?
				.add_output("ssh_load_balancer_dns", terra::Output {
					value: json!(ssh_lb.field_ref("dns_name")),
					description: Some(
						"The DNS name of the SSH network load balancer".into(),
					),
					sensitive: None,
				})?;
		}

		// Outputs
		config
			.add_output("load_balancer_dns", terra::Output {
				value: json!(lb.field_ref("dns_name")),
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


#[cfg(test)]
mod tests {
	use super::*;

	/// Build a config from the given block, returning the config, stack, and the
	/// temp directory guard that keeps the local stack alive.
	fn build_config(block: &FargateBlock) -> (terra::Config, Stack, TempDir) {
		let (stack, dir) = Stack::default_local();
		let mut config = stack.create_config();
		let mut world = World::new();
		block
			.apply_to_config(
				&world.spawn(()).as_readonly(),
				&stack,
				&mut config,
			)
			.unwrap();
		(config, stack, dir)
	}

	#[beet_core::test(timeout_ms = 120000)]
	#[ignore = "very slow"]
	async fn validate() {
		let (config, stack, _dir) = build_config(&FargateBlock::default());
		let project = terra::Project::new(&stack, config);
		project.validate().await.unwrap();
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

	/// The autoscaling-tuned block shared by both ssh states.
	fn autoscaling_block() -> FargateBlock {
		FargateBlock::default()
			.with_min_count(2)
			.with_max_count(7)
			.with_cpu_target_percent(65.0)
	}

	#[beet_core::test]
	fn allow_ssh_emits_ssh_and_autoscaling() {
		let json = build_json(&autoscaling_block().with_allow_ssh(true));
		let ssh_port = beet_net::prelude::DEFAULT_SSH_PORT.to_string();

		xpect_autoscaling(&json);

		// a TCP target group and TCP listener are both present for SSH
		json.matches("\"protocol\":\"TCP\"")
			.count()
			.xpect_greater_or_equal_to(2);

		json
			// SSH network load balancer + exposed ssh port
			.xpect_contains("\"load_balancer_type\":\"network\"")
			.xpect_contains("ssh_load_balancer_dns")
			.xpect_contains("BEET_SSH_PORT")
			.xpect_contains(&ssh_port);
	}

	#[beet_core::test]
	fn http_only_emits_autoscaling_without_ssh() {
		// the default is http-only: autoscaling holds but no ssh infra
		let json = build_json(&autoscaling_block());

		xpect_autoscaling(&json);

		json.xnot()
			.xpect_contains("ssh_load_balancer_dns")
			.xnot()
			.xpect_contains("\"load_balancer_type\":\"network\"")
			.xnot()
			.xpect_contains("BEET_SSH_PORT");
	}
}
