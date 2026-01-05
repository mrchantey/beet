use bevy::prelude::BevyError;

/// Intermediary type for converting to [`BevyError`].
pub struct BevyhowError(pub String);

impl BevyhowError {
	pub fn new(msg: impl Into<String>) -> Self { BevyhowError(msg.into()) }
	pub fn into_bevy(self) -> BevyError { self.into() }
}

impl std::error::Error for BevyhowError {}

impl std::fmt::Display for BevyhowError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.0)
	}
}
impl std::fmt::Debug for BevyhowError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.0)
	}
}

/// A bevy version of [`anyhow::anyhow!`].
#[macro_export]
macro_rules! bevyhow {
		($($arg:tt)*) => {
			$crate::prelude::BevyhowError::new(std::format!($($arg)*)).into_bevy()
		};
}


/// A bevy version of [`anyhow::bail!`].
#[macro_export]
macro_rules! bevybail {
	($($arg:tt)*) => {
		return Err($crate::prelude::BevyhowError::new(std::format!($($arg)*)).into_bevy())
	};
}




#[cfg(test)]
mod test {
	use bevy::ecs::error::BevyError;
	use bevy::prelude::Result;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let foo = 1;
		let bar = 2;
		let a: BevyError = bevyhow!("literal");
		let b: BevyError = bevyhow!("fmt literal inline {foo}{bar}");
		let c: BevyError = bevyhow!("fmt literal {}{}", 1, 2);
		// let d: BevyError = bevyhow!(String::from("expression"));
		a.to_string().xpect_eq("literal\n");
		b.to_string().xpect_eq("fmt literal inline 12\n");
		c.to_string().xpect_eq("fmt literal 12\n");

		let a = || -> Result {
			bevybail!("literal");
		};
		let b = || -> Result {
			bevybail!("fmt literal inline {foo}{bar}");
		};
		let c = || -> Result {
			bevybail!("fmt literal {}{}", 1, 2);
		};

		a().unwrap_err().to_string().xpect_eq("literal\n");
		b().unwrap_err()
			.to_string()
			.xpect_eq("fmt literal inline 12\n");
		c().unwrap_err().to_string().xpect_eq("fmt literal 12\n");
	}
}
