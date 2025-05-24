use crate::prelude::*;
use std::fmt;
use std::iter::FromIterator;

#[derive(Default, Debug, PartialEq)]
pub struct RsxAttributes(pub Box<[RsxAttribute]>);

impl From<Option<RsxAttributes>> for RsxAttributes {
	fn from(children: Option<RsxAttributes>) -> Self {
		children.unwrap_or_default()
	}
}

impl From<Vec<RsxAttribute>> for RsxAttributes {
	fn from(vec: Vec<RsxAttribute>) -> Self {
		RsxAttributes(vec.into_boxed_slice())
	}
}

impl FromIterator<RsxAttribute> for RsxAttributes {
	fn from_iter<I: IntoIterator<Item = RsxAttribute>>(iter: I) -> Self {
		RsxAttributes::from(iter.into_iter().collect::<Vec<_>>())
	}
}

#[derive(Debug, PartialEq)]
pub enum RsxAttribute {
	Named(RsxAttributeName, RsxAttributeValue),
	Spread(RsxParsedExpression),
}

#[derive(Debug, PartialEq)]
pub enum RsxAttributeName {
	Name(RsxIdentifier),
}


impl fmt::Display for RsxAttributeName {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		use self::RsxAttributeName::*;
		match self {
			&Name(ref n) => write!(f, "{}", n.0),
		}
	}
}

#[derive(Debug, PartialEq)]
pub enum RsxAttributeValue {
	/// no value
	Default,
	Boolean(RsxAttributeBoolean),
	Number(RsxAttributeNumber),
	Str(RsxAttributeString),
	Element(RsxElement),
	CodeBlock(RsxParsedExpression),
}

#[derive(Debug, PartialEq)]
pub struct RsxAttributeBoolean(pub bool);

impl From<JSBool> for RsxAttributeBoolean {
	fn from(v: JSBool) -> Self { RsxAttributeBoolean(v.0) }
}

impl std::fmt::Display for RsxAttributeBoolean {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{}", self.0)
	}
}

#[derive(Debug, PartialEq)]
pub struct RsxAttributeNumber(pub f64);

impl std::fmt::Display for RsxAttributeNumber {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{}", self.0)
	}
}

impl From<JSNumber> for RsxAttributeNumber {
	fn from(n: JSNumber) -> Self { RsxAttributeNumber(n.0) }
}

#[derive(Debug, PartialEq)]
pub enum RsxAttributeString {
	SingleQuoted(JSSingleStringCharacters),
	DoubleQuoted(JSDoubleStringCharacters),
}

impl std::fmt::Display for RsxAttributeString {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		use self::RsxAttributeString::*;
		match self {
			&SingleQuoted(ref s) => write!(f, "'{}'", s.0),
			&DoubleQuoted(ref s) => write!(f, "\"{}\"", s.0),
		}
	}
}


impl RsxAttributeString {
	/// Returns the string without quotes, useful for comparisons.
	pub fn to_string_unquoted(&self) -> String {
		match self {
			RsxAttributeString::SingleQuoted(v) => v.0.to_string(),
			RsxAttributeString::DoubleQuoted(v) => v.0.to_string(),
		}
	}
}
