use crate::bindings::*;
use crate::prelude::*;
use crate::terra::ResourceDef;
use beet_core::prelude::*;
use serde_json::json;

/// A DNS record provider, embedded in a block that needs to publish a hostname
/// (a [`LambdaBlock`] gateway, a [`FargateBlock`] load balancer, a
/// [`LightsailBlock`] static IP). It emits a single record pointing its
/// `authority` at a target: a `CNAME` via [`emit`] for hostname targets, an
/// `A`/`AAAA` via [`emit_address`] for IP targets, plus any auxiliary records
/// (eg ACM DNS-validation) via [`emit_validation_record`].
///
/// [`emit`]: Self::emit
/// [`emit_address`]: Self::emit_address
///
/// [`emit_validation_record`]: Self::emit_validation_record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DnsProvider {
	/// A record in a Cloudflare zone. Authenticates from the
	/// `CLOUDFLARE_API_TOKEN` environment variable at apply time.
	#[cfg(feature = "cloudflare_dns")]
	Cloudflare {
		/// Fully-qualified record name, eg `dev.beet.org`.
		authority: SmolStr,
		/// The Cloudflare zone id (from `CLOUDFLARE_ZONE_ID`).
		zone_id: SmolStr,
		/// Whether to proxy through Cloudflare's edge. DNS-only (`false`) is
		/// required when the origin must be reached directly, eg raw TCP ssh
		/// or terminating TLS at the origin.
		proxied: bool,
	},
	/// A record in a Route53 hosted zone.
	Route53 {
		/// Fully-qualified record name.
		authority: SmolStr,
		/// The Route53 hosted zone id.
		zone_id: SmolStr,
	},
}

impl DnsProvider {
	/// A Cloudflare record, DNS-only (not proxied) by default.
	#[cfg(feature = "cloudflare_dns")]
	pub fn cloudflare(
		authority: impl Into<SmolStr>,
		zone_id: impl Into<SmolStr>,
	) -> Self {
		Self::Cloudflare {
			authority: authority.into(),
			zone_id: zone_id.into(),
			proxied: false,
		}
	}

	/// A Route53 record.
	pub fn route53(
		authority: impl Into<SmolStr>,
		zone_id: impl Into<SmolStr>,
	) -> Self {
		Self::Route53 {
			authority: authority.into(),
			zone_id: zone_id.into(),
		}
	}

	/// Proxy a Cloudflare record through the edge (no effect on Route53).
	#[cfg(feature = "cloudflare_dns")]
	pub fn with_proxied(mut self, value: bool) -> Self {
		if let Self::Cloudflare { proxied, .. } = &mut self {
			*proxied = value;
		}
		self
	}

	/// The record name this provider publishes, eg `dev.beet.org`.
	pub fn authority(&self) -> &SmolStr {
		match self {
			#[cfg(feature = "cloudflare_dns")]
			Self::Cloudflare { authority, .. } => authority,
			Self::Route53 { authority, .. } => authority,
		}
	}

	/// The zone id the records are emitted into.
	pub fn zone_id(&self) -> &SmolStr {
		match self {
			#[cfg(feature = "cloudflare_dns")]
			Self::Cloudflare { zone_id, .. } => zone_id,
			Self::Route53 { zone_id, .. } => zone_id,
		}
	}

	/// Emit a `CNAME` pointing [`authority`](Self::authority) at `alias_target`
	/// (a terra field-ref like a load balancer's `dns_name` or an api gateway's
	/// `api_endpoint`). `label` is the resource label suffix.
	pub fn emit(
		&self,
		stack: &Stack,
		config: &mut terra::Config,
		label: &str,
		alias_target: &str,
	) -> Result {
		self.emit_record(
			stack,
			config,
			label,
			self.authority(),
			alias_target,
			self.proxied(),
			"CNAME",
		)?;
		Ok(())
	}

	/// Emit an address record pointing [`authority`](Self::authority) at
	/// `address` (a terra field-ref resolving to an IP, eg a Lightsail static
	/// IP's `ip_address`): an `AAAA` when `ipv6`, else an `A`.
	pub fn emit_address(
		&self,
		stack: &Stack,
		config: &mut terra::Config,
		label: &str,
		address: &str,
		ipv6: bool,
	) -> Result {
		self.emit_record(
			stack,
			config,
			label,
			self.authority(),
			address,
			self.proxied(),
			if ipv6 { "AAAA" } else { "A" },
		)?;
		Ok(())
	}

	/// Whether records emit proxied through Cloudflare's edge.
	fn proxied(&self) -> bool {
		#[cfg(feature = "cloudflare_dns")]
		return matches!(self, Self::Cloudflare { proxied: true, .. });
		#[cfg(not(feature = "cloudflare_dns"))]
		false
	}

	/// Emit an ACM DNS-validation `CNAME` (always unproxied) into this
	/// provider's zone, pointing `name` at `content` (terra field-refs read off
	/// the certificate's `domain_validation_options`). Returns the terraform
	/// resource address for use in a validation resource's `depends_on`.
	pub fn emit_validation_record(
		&self,
		stack: &Stack,
		config: &mut terra::Config,
		label: &str,
		name: &str,
		content: &str,
	) -> Result<String> {
		self.emit_record(stack, config, label, name, content, false, "CNAME")
	}

	/// Emit one record of `record_type` into this provider's zone, returning its
	/// terraform resource address (eg `cloudflare_dns_record.<label>`).
	fn emit_record(
		&self,
		stack: &Stack,
		config: &mut terra::Config,
		label: &str,
		name: &str,
		content: &str,
		proxied: bool,
		record_type: &str,
	) -> Result<String> {
		let ident = stack.resource_ident(label);
		let address = match self {
			#[cfg(feature = "cloudflare_dns")]
			Self::Cloudflare { zone_id, .. } => {
				ensure_cloudflare_provider(config)?;
				let record = ResourceDef::new_secondary(
					ident,
					CloudflareDnsRecordDetails {
						name: name.into(),
						ttl: 1,
						r#type: record_type.into(),
						zone_id: zone_id.clone(),
						content: Some(content.into()),
						proxied: Some(proxied),
						..default()
					},
				);
				let address =
					format!("cloudflare_dns_record.{}", record.ident().label());
				config.add_resource(&record)?;
				address
			}
			Self::Route53 { zone_id, .. } => {
				let record = ResourceDef::new_secondary(
					ident,
					AwsRoute53RecordDetails {
						name: name.into(),
						r#type: record_type.into(),
						zone_id: zone_id.clone(),
						ttl: Some(60),
						records: Some(vec![content.into()]),
						..default()
					},
				);
				let address =
					format!("aws_route53_record.{}", record.ident().label());
				config.add_resource(&record)?;
				address
			}
		};
		Ok(address)
	}
}

/// Ensure the Cloudflare terraform provider is configured. The block stays
/// empty: the provider authenticates from `CLOUDFLARE_API_TOKEN` in the
/// environment (inherited by the tofu subprocess), keeping the secret out of
/// `main.tf.json`.
#[cfg(feature = "cloudflare_dns")]
pub(crate) fn ensure_cloudflare_provider(config: &mut terra::Config) -> Result {
	config.ensure_provider_config(&terra::Provider::CLOUDFLARE, &json!({}))?;
	Ok(())
}
