/// Specify the environment of the api
/// 
/// The default value will be resolved by [`ApiEnvironment::get`]
/// 1. if `API_ENV` is set to local,staging,prod, use that value
/// 2. Otherwise use `debug_assertions` to determine if it should be local or prod
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiEnvironment {
	/// Should not use Mongo or AWS at all
	Local,
	/// Use remote services but staging buckets and dbs
	Staging,
	/// Use production remote services
	Prod,
}

impl Default for ApiEnvironment {
	fn default() -> Self { Self::get() }
}

impl Into<&'static str> for ApiEnvironment {
	fn into(self) -> &'static str {
		match self {
			Self::Local => "local",
			Self::Staging => "staging",
			Self::Prod => "prod",
		}
	}
}

impl std::fmt::Display for ApiEnvironment {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", Into::<&'static str>::into(*self))
	}
}


impl ApiEnvironment {
	pub fn get() -> Self {
		if let Some(env) = std::env::var("API_ENV").ok() {
			match env.as_str() {
				"local" => Self::Local,
				"staging" => Self::Staging,
				"prod" => Self::Prod,
				_ => Self::from_debug_assertions(),
			}
		} else {
			Self::from_debug_assertions()
		}
	}
	fn from_debug_assertions() -> Self {
		if cfg!(debug_assertions) {
			Self::Local
		} else {
			Self::Prod
		}
	}
	pub fn is_local(&self) -> bool { *self == Self::Local }
}
