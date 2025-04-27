use sea_query::*;
use std::borrow::Cow;
use std::fmt;



// pub struct StaticIden(pub &'static str);

// impl StaticIden {
// 	/// Creates a new `CowIden` from a `&'static str`.
// 	pub fn new(s: &'static str) -> Self { StaticIden(s) }
// }

// impl Iden for StaticIden {
// 	fn unquoted(&self, s: &mut dyn fmt::Write) {
// 		s.write_str(&self.0).unwrap();
// 	}
// }


/// A wrapper around `Cow<'static, str>` that implements the [`sea_query::Iden`] trait.
pub struct CowIden(pub Cow<'static, str>);
impl CowIden {
	/// Creates a new `CowIden` from a `Cow<'static, str>`.
	pub fn new(s: impl Into<Cow<'static, str>>) -> Self { CowIden(s.into()) }
}

impl Iden for CowIden {
	fn unquoted(&self, s: &mut dyn fmt::Write) {
		s.write_str(&self.0).unwrap();
	}
}
