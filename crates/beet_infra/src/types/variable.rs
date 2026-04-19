//! Variables for passing values to tofu commands.
use beet_core::prelude::*;
use beet_net::prelude::*;

/// A variable to be passed to tofu commands, commonly used
/// for inserting application-specific environment variables in runtimes,
/// catching missing variables before deploy.
#[derive(Debug, Clone, Get, Serialize, Deserialize)]
pub struct Variable {
	key: SmolStr,
	value: VariableValue,
}

/// How a [`Variable`] value is resolved at deploy time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VariableValue {
	/// A fixed literal value.
	Fixed(SmolStr),
	/// Collected from the deployer's process environment via [`env_ext::var`].
	ProcessEnv,
	/// Collected from the request headers, see [`RequestParts::headers`].
	Header,
	/// Collected from the request params, see [`RequestParts::params`].
	Param,
}

impl Variable {
	/// Create a variable with a fixed literal value.
	pub fn fixed(key: impl Into<SmolStr>, value: impl Into<SmolStr>) -> Self {
		Self {
			key: key.into(),
			value: VariableValue::Fixed(value.into()),
		}
	}

	/// Create a variable resolved from the deployer's process environment.
	pub fn process_env(key: impl Into<SmolStr>) -> Self {
		Self {
			key: key.into(),
			value: VariableValue::ProcessEnv,
		}
	}

	/// Create a variable resolved from request headers.
	pub fn header(key: impl Into<SmolStr>) -> Self {
		Self {
			key: key.into(),
			value: VariableValue::Header,
		}
	}

	/// Create a variable resolved from request params.
	pub fn param(key: impl Into<SmolStr>) -> Self {
		Self {
			key: key.into(),
			value: VariableValue::Param,
		}
	}

	/// Resolve the variable value from the given request context.
	pub fn resolve_value(&self, request: &RequestParts) -> Result<SmolStr> {
		match &self.value {
			VariableValue::Fixed(value) => Ok(value.clone()),
			VariableValue::ProcessEnv => env_ext::var(self.key.as_str())
				.map(SmolStr::new)
				.map_err(|_| {
					bevyhow!("process env variable '{}' not found", self.key)
				}),
			VariableValue::Header => request
				.headers()
				.first_raw(self.key.as_str())
				.map(SmolStr::new)
				.ok_or_else(|| {
					bevyhow!("header variable '{}' not found", self.key)
				}),
			VariableValue::Param => request
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
			// none even for fixed, this is more consistent
			// they will definitely be inserted at the apply step,
			// and any missing variables is a pre-apply error
			default: None,
			description: Some(format!("Variable: {}", self.key)),
		}
	}
}


#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn fixed_variable() {
		let var = Variable::fixed("MY_KEY", "my_value");
		let request = RequestParts::default();
		var.resolve_value(&request)
			.unwrap()
			.as_str()
			.xpect_eq("my_value");
	}

	#[test]
	fn header_variable() {
		let var = Variable::header("x-api-key");
		let mut request = RequestParts::default();
		request.headers_mut().set_raw("x-api-key", "secret123");
		var.resolve_value(&request)
			.unwrap()
			.as_str()
			.xpect_eq("secret123");
	}

	#[test]
	fn param_variable() {
		let var = Variable::param("deploy_env");
		let request =
			RequestParts::default().with_param("deploy_env", "staging");
		var.resolve_value(&request)
			.unwrap()
			.as_str()
			.xpect_eq("staging");
	}

	#[test]
	fn missing_header_errors() {
		let var = Variable::header("missing-key");
		let request = RequestParts::default();
		var.resolve_value(&request).unwrap_err();
	}

	#[test]
	fn missing_param_errors() {
		let var = Variable::param("missing-key");
		let request = RequestParts::default();
		var.resolve_value(&request).unwrap_err();
	}

	#[test]
	fn tf_var_ref_format() {
		let var = Variable::fixed("MY_KEY", "val");
		var.tf_var_ref().as_str().xpect_eq("${var.MY_KEY}");
	}
}
