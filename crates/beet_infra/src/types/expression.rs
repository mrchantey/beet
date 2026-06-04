use beet_core::prelude::*;
use beet_net::prelude::*;

// This is a bit of an antipattern, we generate config files
// on the fly so leaning into terraform variables, functions etc
// is against the grain of the rust-only wrapper approach.
pub enum Expression {
	Null,
	Bool(bool),
	Number(f64),
	String(String),
	Array(Vec<Expression>),
	Variable(String),
	Func { name: String, args: Vec<Expression> },
}

impl Expression {
	pub fn new(val: impl Into<Self>) -> Self { val.into() }

	/// convert this expression into a terraform string,
	/// with `${}` escaping where required.
	pub fn into_terra(self) -> String {
		match &self {
			Self::Null | Self::Bool(_) | Self::Number(_) | Self::String(_) => {
				// no escape required
				self.into_terra_inner()
			}
			Self::Variable(_) | Self::Func { .. } | Self::Array(_) => {
				// these types need escaping
				format!("${{{}}}", self.into_terra_inner())
			}
		}
	}
	/// convert this expression into a terraform string,
	/// without any `${}` escape.
	fn into_terra_inner(self) -> String {
		match self {
			Self::Null => "null".into(),
			Self::Bool(b) => b.to_string(),
			Self::Number(n) => n.to_string(),
			Self::String(s) => format!("\"{}\"", s),
			Self::Array(arr) => {
				let inner = arr
					.into_iter()
					.map(|e| e.into_terra_inner())
					.collect::<Vec<_>>()
					.join(", ");
				format!("[{}]", inner)
			}
			Self::Variable(var) => format!("var.{}", var),
			Self::Func { name, args } => {
				let args = args
					.into_iter()
					.map(|e| e.into_terra_inner())
					.collect::<Vec<_>>()
					.join(", ");
				format!("{}({})", name, args)
			}
		}
	}
}

impl Expression {
	pub fn join(seperator: impl Into<Self>, items: impl Into<Self>) -> Self {
		Self::Func {
			name: "join".into(),
			args: vec![seperator.into(), items.into()],
		}
	}
	pub fn compact(items: impl Into<Self>) -> Self {
		Self::Func {
			name: "compact".into(),
			args: vec![items.into()],
		}
	}
}

impl From<bool> for Expression {
	fn from(b: bool) -> Self { Self::Bool(b) }
}
impl From<f64> for Expression {
	fn from(n: f64) -> Self { Self::Number(n) }
}
impl From<String> for Expression {
	fn from(s: String) -> Self { Self::String(s) }
}
impl From<&str> for Expression {
	fn from(s: &str) -> Self { Self::String(s.into()) }
}
impl<T> From<Vec<T>> for Expression
where
	T: Into<Self>,
{
	fn from(vec: Vec<T>) -> Self {
		Self::Array(vec.into_iter().map(|item| item.into()).collect())
	}
}

impl From<PathPatternSegment> for Expression {
	fn from(seg: PathPatternSegment) -> Self {
		match seg.modifier() {
			PathPatternModifier::Static => Self::String(seg.name().to_string()),
			PathPatternModifier::Required => {
				Self::Variable(seg.name().to_string())
			}
			PathPatternModifier::Optional => {
				Self::compact(Self::Variable(seg.name().to_string()))
			}
			PathPatternModifier::OneOrMore
			| PathPatternModifier::ZeroOrMore => {
				Self::join("--", Self::Variable(seg.name().to_string()))
			}
		}
	}
}
