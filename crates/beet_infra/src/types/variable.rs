//! Variables for passing values to tofu commands.
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// A variable to be passed to tofu commands, commonly used
/// for inserting application-specific environment variables in runtimes,
/// catching missing variables before deploy.
///
/// Three independent things: where the value comes from
/// ([`VariableSource`]), whether terraform may print it (`sensitive`, a
/// property of the use, never implied by the source: the two store-backed
/// variables today are public halves), and whether absence is a legal state,
/// which only the store source can answer and carries itself.
#[derive(Debug, Clone, Get, SetWith, Serialize, Deserialize, Reflect)]
pub struct Variable {
	key: SmolStr,
	source: VariableSource,
	/// Redact this value everywhere tofu would otherwise print it: the plan, the
	/// apply output, an `output` that reads it. State is NOT covered, which is
	/// why a stack declaring one turns on state encryption.
	sensitive: bool,
}

/// Where a [`Variable`]'s value comes from at deploy time. Two families:
/// the ambient sources (`ProcessEnv`, `Header`, `Param`) a pipeline step
/// supplies at apply, harmless with an empty default; and the content
/// sources (`Fixed`, `Secret`) every rendering verb resolves, since their
/// value decides what a resource IS.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub enum VariableSource {
	/// A fixed literal value.
	Fixed(SmolStr),
	/// Collected from the deployer's process environment via [`env_ext::var`].
	ProcessEnv,
	/// Collected from the request headers, see [`RequestParts::headers`].
	Header,
	/// Collected from the request params, see [`RequestParts::params`].
	Param,
	/// Read from the stack's secret store at the named secret, every time the
	/// config is rendered.
	///
	/// The others are AMBIENT: a pipeline step supplies them and an empty
	/// default is harmless, because they feed runtime attributes (a lambda's
	/// env) rather than what a resource IS. This one is the opposite, and the
	/// distinction is load-bearing: it exists for a value that is the CONTENT of
	/// a resource, where a wrong value is not a missing feature but a broken
	/// one. A DKIM selector record holding `p=` does not mean "no key", it means
	/// "this key is revoked", so every message the domain signs fails.
	///
	/// Hence: no default, resolved by every verb that renders, and a hard error
	/// when the secret is absent, unless `optional`.
	Secret {
		/// The secret, by label; the store composes the address.
		secret: SecretRef,
		/// Whether ABSENCE is a legal state: the secret not existing resolves
		/// to empty rather than refusing, and the block reading it emits
		/// nothing for empty (a `count` on the value). The store is the one
		/// source where absence is a runtime state that changes over time,
		/// which is why this lives here and not on every variable.
		///
		/// The case: a `TLSA` pinning the certificate a server serves. Nothing
		/// has been issued on a fresh stack, so there is nothing to pin and no
		/// record is the right record; the value is parked once the server
		/// serves one, and the next render publishes it. Still content (every
		/// verb that renders reads it, so a `plan` is truthful) and still no
		/// terraform default, so a bare invocation outside these verbs refuses
		/// rather than withdrawing the record.
		optional: bool,
	},
}

impl VariableSource {
	/// The value of a `Fixed` source, or of a `ProcessEnv` one read now: the
	/// two a block resolves at render without a request or a store, ie a
	/// unit's `Environment=` line. A process variable unset at render is
	/// empty with a warning naming `key`, so a machine without the secret
	/// still renders (the box then falls back to whatever the runtime does
	/// without it). Any other source is an error.
	pub fn resolve_static(&self, key: &str) -> Result<SmolStr> {
		match self {
			Self::Fixed(value) => Ok(value.clone()),
			Self::ProcessEnv => env_ext::var(key).map(SmolStr::new).or_else(|_| {
				warn!(
					"`{key}` is unset in the process environment, rendering it \
					empty"
				);
				Ok(SmolStr::default())
			}),
			Self::Header | Self::Param | Self::Secret { .. } => bevybail!(
				"`{key}` reads {self:?}, which cannot be resolved at render \
				without a request"
			),
		}
	}
}

impl Variable {
	/// Create a variable with a fixed literal value.
	pub fn fixed(key: impl Into<SmolStr>, value: impl Into<SmolStr>) -> Self {
		Self {
			key: key.into(),
			source: VariableSource::Fixed(value.into()),
			sensitive: false,
		}
	}

	/// Create a variable resolved from the deployer's process environment.
	pub fn process_env(key: impl Into<SmolStr>) -> Self {
		Self {
			key: key.into(),
			source: VariableSource::ProcessEnv,
			sensitive: false,
		}
	}

	/// Create a variable resolved from request headers.
	pub fn header(key: impl Into<SmolStr>) -> Self {
		Self {
			key: key.into(),
			source: VariableSource::Header,
			sensitive: false,
		}
	}

	/// Create a variable resolved from request params.
	pub fn param(key: impl Into<SmolStr>) -> Self {
		Self {
			key: key.into(),
			source: VariableSource::Param,
			sensitive: false,
		}
	}

	/// Create a variable read from the stack's secret store, see
	/// [`VariableSource::Secret`].
	pub fn secret(key: impl Into<SmolStr>, secret: SecretRef) -> Self {
		Self {
			key: key.into(),
			source: VariableSource::Secret {
				secret,
				optional: false,
			},
			sensitive: false,
		}
	}

	/// Create a variable read from the stack's secret store whose absence
	/// resolves to empty, see [`VariableSource::Secret`].
	pub fn secret_optional(key: impl Into<SmolStr>, secret: SecretRef) -> Self {
		Self {
			key: key.into(),
			source: VariableSource::Secret {
				secret,
				optional: true,
			},
			sensitive: false,
		}
	}

	/// Whether this variable's value is the CONTENT of a resource rather than an
	/// ambient runtime attribute, so it carries no default and must resolve
	/// before any render.
	pub fn is_content(&self) -> bool {
		matches!(self.source, VariableSource::Secret { .. })
	}

	/// Whether an absent secret resolves to empty rather than refusing,
	/// see [`VariableSource::Secret`].
	pub fn absent_is_empty(&self) -> bool {
		matches!(self.source, VariableSource::Secret { optional: true, .. })
	}

	/// The secret this variable reads, if its source is the store.
	///
	/// The READ itself belongs to the deploy side (`terra::Project`, through
	/// the stack's [`SecretStore`](crate::prelude::StackQuery::secret_store)),
	/// not here: this type is compiled into every build that describes
	/// infrastructure, including the deployed binary, which links no
	/// provider at all. A declaration should not drag in the machinery that
	/// acts on it.
	pub fn secret_ref(&self) -> Option<&SecretRef> {
		match &self.source {
			VariableSource::Secret { secret, .. } => Some(secret),
			_ => None,
		}
	}

	/// A fixed value, if this variable carries one, so a render can resolve it
	/// without a request.
	pub fn fixed_value(&self) -> Option<&SmolStr> {
		match &self.source {
			VariableSource::Fixed(value) => Some(value),
			_ => None,
		}
	}

	/// Resolve the variable value from the given request context.
	///
	/// The request-bound counterpart of [`resolve_for_render`](Self::resolve_for_render),
	/// used by a deploy pipeline where an earlier step has supplied the ambient
	/// values as params. A [`Secret`](VariableSource::Secret) variable is not
	/// resolvable from a request and is an error here; the render resolves it
	/// through the stack's secret store.
	pub fn resolve_value(&self, request: &RequestParts) -> Result<SmolStr> {
		match &self.source {
			VariableSource::Fixed(value) => Ok(value.clone()),
			VariableSource::Secret { secret, .. } => bevybail!(
				"variable `{}` reads secret `{}` and is not resolvable from a \
				request",
				self.key,
				secret.label()
			),
			VariableSource::ProcessEnv => env_ext::var(self.key.as_str())
				.map(SmolStr::new)
				.map_err(|_| {
					bevyhow!("process env variable '{}' not found", self.key)
				}),
			VariableSource::Header => request
				.headers()
				.first_raw(self.key.as_str())
				.map(SmolStr::new)
				.ok_or_else(|| {
					bevyhow!("header variable '{}' not found", self.key)
				}),
			VariableSource::Param => request
				.get_param(self.key.as_str())
				.map(SmolStr::new)
				.ok_or_else(|| {
					bevyhow!("param variable '{}' not found", self.key)
				}),
		}
	}

	/// The terraform variable reference expression for use in config,
	/// ie `${var.my_key}`.
	pub fn tf_var_ref(&self) -> String { format!("${{var.{}}}", self.key) }

	/// Build the [`terra::Variable`](crate::terra::config::Variable) declaration
	/// for this variable in the terraform config.
	pub fn tf_declaration(&self) -> crate::terra::Variable {
		crate::terra::Variable {
			r#type: Some("string".into()),
			// an empty default so `destroy`/`plan` (which pass no `-var`) succeed:
			// these values only feed runtime resource attributes (eg a lambda's env),
			// never resource identity, so the default is irrelevant to a teardown.
			// `apply` always overrides it with the resolved value via `-var`.
			//
			// A CONTENT variable gets no default at all: an empty value there is
			// not a harmless placeholder but a wrong resource, so terraform
			// should refuse rather than render it. Every verb that renders
			// resolves it first (`resolve_for_render`), so the only way to reach
			// the refusal is a value that genuinely is not there yet.
			default: (!self.is_content()).then(|| "".into()),
			description: Some(format!("Variable: {}", self.key)),
			sensitive: self.sensitive.then_some(true),
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[beet_core::test]
	fn fixed_variable() {
		let var = Variable::fixed("MY_KEY", "my_value");
		let request = RequestParts::default();
		var.resolve_value(&request)
			.unwrap()
			.as_str()
			.xpect_eq("my_value");
	}

	#[beet_core::test]
	fn header_variable() {
		let var = Variable::header("x-api-key");
		let mut request = RequestParts::default();
		request.headers_mut().set_raw("x-api-key", "secret123");
		var.resolve_value(&request)
			.unwrap()
			.as_str()
			.xpect_eq("secret123");
	}

	#[beet_core::test]
	fn param_variable() {
		let var = Variable::param("deploy_env");
		let request =
			RequestParts::default().with_param("deploy_env", "staging");
		var.resolve_value(&request)
			.unwrap()
			.as_str()
			.xpect_eq("staging");
	}

	#[beet_core::test]
	fn missing_header_errors() {
		let var = Variable::header("missing-key");
		let request = RequestParts::default();
		var.resolve_value(&request).unwrap_err();
	}

	#[beet_core::test]
	fn missing_param_errors() {
		let var = Variable::param("missing-key");
		let request = RequestParts::default();
		var.resolve_value(&request).unwrap_err();
	}

	#[beet_core::test]
	fn tf_var_ref_format() {
		let var = Variable::fixed("MY_KEY", "val");
		var.tf_var_ref().as_str().xpect_eq("${var.MY_KEY}");
	}

	// an empty default so `tofu destroy`/`plan` need no `-var`; `apply` overrides it.
	#[beet_core::test]
	fn tf_declaration_has_empty_default() {
		Variable::fixed("MY_KEY", "val")
			.tf_declaration()
			.default
			.xpect_eq(Some("".into()));
	}

	/// Both secret flavours are content, so every verb that renders resolves
	/// them and neither carries a terraform default; only the optional one
	/// lets an absent secret through as empty.
	#[beet_core::test]
	fn optional_secret_is_content_without_a_default() {
		let optional =
			Variable::secret_optional("mail_tlsa", SecretRef::new("tlsa"));
		optional.is_content().xpect_true();
		optional.absent_is_empty().xpect_true();
		optional.tf_declaration().default.xpect_eq(None);
		optional
			.secret_ref()
			.unwrap()
			.label()
			.as_str()
			.xpect_eq("tlsa");
		Variable::secret("dkim", SecretRef::new("dkim"))
			.absent_is_empty()
			.xpect_false();
		optional
			.resolve_value(&RequestParts::default())
			.unwrap_err()
			.to_string()
			.xpect_contains("secret `tlsa`");
	}

	// `sensitive` is omitted rather than declared false, so the emitted json is
	// the same as it was before secrets rode this type at all.
	#[beet_core::test]
	fn sensitive_is_opt_in() {
		Variable::param("db_password")
			.tf_declaration()
			.sensitive
			.xpect_eq(None);
		Variable::param("db_password")
			.with_sensitive(true)
			.tf_declaration()
			.sensitive
			.xpect_eq(Some(true));
	}
}
