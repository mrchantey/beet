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
	($msg:literal $(,)?) => {
		$crate::prelude::BevyhowError::new($msg).into_bevy()
	};
	($fmt:expr, $($arg:tt)*) => {
		$crate::prelude::BevyhowError::new(std::format!($fmt, $($arg)*)).into_bevy()
		};
		($err:expr $(,)?) => {
			$crate::prelude::BevyhowError::new($err).into_bevy()
		};
}

/// A bevy version of [`anyhow::bail!`].
#[macro_export]
macro_rules! bevybail {
	($msg:literal $(,)?) => {
		return Err($crate::prelude::BevyhowError::new($msg).into_bevy())
	};
	($fmt:expr, $($arg:tt)*) => {
		return Err($crate::prelude::BevyhowError::new(std::format!($fmt, $($arg)*)).into_bevy())
	};
	($err:expr $(,)?) => {
		return Err($crate::prelude::BevyhowError::new($err).into_bevy())
	};
}




#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::ecs::error::BevyError;
	use bevy::prelude::Result;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let a: BevyError = bevyhow!("literal");
		let c: BevyError = bevyhow!(String::from("expression"));
		let b: BevyError = bevyhow!("fmt literal {}{}", 1, 2);
		expect(a.to_string()).to_be("literal\n");
		expect(b.to_string()).to_be("fmt literal 12\n");
		expect(c.to_string()).to_be("expression\n");

		let a = || -> Result {
			bevybail!("literal");
		};
		let b = || -> Result {
			bevybail!("fmt literal {}{}", 1, 2);
		};
		let c = || -> Result {
			bevybail!(String::from("expression"));
		};
		expect(a().unwrap_err().to_string()).to_be("literal\n");
		expect(b().unwrap_err().to_string()).to_be("fmt literal 12\n");
		expect(c().unwrap_err().to_string()).to_be("expression\n");
	}
}
