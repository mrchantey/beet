use crate::bindings::*;
use crate::prelude::*;
use crate::terra::ResourceDef;
use beet_core::prelude::*;
use serde_json::json;

/// A DNS record provider, embedded in a block that needs to publish a hostname
/// (a [`LambdaBlock`] gateway, a [`FargateBlock`] load balancer). It emits a
/// single `CNAME` pointing its `authority` at an alias target, plus any
/// auxiliary records (eg ACM DNS-validation) via [`emit_validation_record`].
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
		/// without a Spectrum app, or terminating TLS at the origin.
		proxied: bool,
		/// Publish a Spectrum app forwarding raw-TCP port 22 to the origin, so
		/// `ssh <authority>` keeps working while the record is proxied (the
		/// HTTP proxy only carries HTTP ports). Declarative metadata read by
		/// the `CloudflareZoneSetup` action, which manages the app through the
		/// zone API: terraform cannot manage a non-Enterprise Spectrum app
		/// (the provider always sends Enterprise-only fields, which the
		/// plan-polymorphic API rejects). Spectrum's SSH preset needs a paid
		/// plan (Pro+), is port-22 only, and takes only direct IP origins, so
		/// the owning [`FargateBlock`] gives its NLB static EIPs.
		ssh_spectrum: bool,
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
			ssh_spectrum: false,
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

	/// Carry raw-TCP ssh through a Spectrum app on a proxied Cloudflare record
	/// (no effect on Route53), see [`Cloudflare::ssh_spectrum`](Self::Cloudflare).
	#[cfg(feature = "cloudflare_dns")]
	pub fn with_ssh_spectrum(mut self, value: bool) -> Self {
		if let Self::Cloudflare { ssh_spectrum, .. } = &mut self {
			*ssh_spectrum = value;
		}
		self
	}

	/// Whether this hostname carries ssh through a Spectrum app.
	pub fn ssh_spectrum(&self) -> bool {
		#[cfg(feature = "cloudflare_dns")]
		if let Self::Cloudflare { ssh_spectrum, .. } = self {
			return *ssh_spectrum;
		}
		false
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
	/// `api_endpoint`). `label` is the resource label suffix. An
	/// [`ssh_spectrum`](Self::ssh_spectrum) hostname's Spectrum app is not
	/// terraform (see the field docs): the `CloudflareZoneSetup` action
	/// publishes it after the apply.
	pub fn emit(
		&self,
		stack: &Stack,
		config: &mut terra::Config,
		label: &str,
		alias_target: &str,
	) -> Result {
		#[cfg(feature = "cloudflare_dns")]
		let proxied = matches!(self, Self::Cloudflare { proxied: true, .. });
		#[cfg(not(feature = "cloudflare_dns"))]
		let proxied = false;
		self.emit_record(
			stack,
			config,
			label,
			self.authority(),
			alias_target,
			proxied,
		)?;
		Ok(())
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
		self.emit_record(stack, config, label, name, content, false)
	}

	/// Emit one `CNAME` into this provider's zone, returning its terraform
	/// resource address (eg `cloudflare_dns_record.<label>`).
	fn emit_record(
		&self,
		stack: &Stack,
		config: &mut terra::Config,
		label: &str,
		name: &str,
		content: &str,
		proxied: bool,
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
						r#type: "CNAME".into(),
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
						r#type: "CNAME".into(),
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
pub fn ensure_cloudflare_provider(config: &mut terra::Config) -> Result {
	config.ensure_provider_config(&terra::Provider::CLOUDFLARE, &json!({}))?;
	Ok(())
}
