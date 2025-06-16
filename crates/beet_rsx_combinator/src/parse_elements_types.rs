use std::fmt;
use std::iter::FromIterator;

use crate::prelude::*;
use itertools::Itertools;

#[derive(Debug, PartialEq)]
pub enum RsxElement {
	SelfClosing(RsxSelfClosingElement),
	Normal(RsxNormalElement),
	Fragment(RsxFragment),
}

#[derive(Debug, PartialEq)]
pub struct RsxFragment(pub RsxChildren);

#[derive(Debug, PartialEq)]
pub struct RsxSelfClosingElement(pub RsxElementName, pub RsxAttributes);

#[derive(Debug, PartialEq)]
pub struct RsxNormalElement(
	pub RsxElementName,
	pub RsxAttributes,
	pub RsxChildren,
);

#[derive(Debug, PartialEq)]
pub struct RsxOpeningElement(pub RsxElementName, pub RsxAttributes);

#[derive(Debug, PartialEq)]
pub struct RsxClosingElement<'a>(pub &'a RsxElementName);

#[derive(Debug, PartialEq)]
pub enum RsxElementName {
	Name(RsxIdentifier),
	NamedspacedName(RsxIdentifier, RsxIdentifier),
	MemberExpression(Box<[RsxIdentifier]>),
}

impl fmt::Display for RsxElementName {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		use self::RsxElementName::*;
		match self {
			&Name(ref n) => write!(f, "{}", n.0),
			&NamedspacedName(ref ns, ref n) => write!(f, "{}:{}", ns.0, n.0),
			&MemberExpression(ref e) => {
				write!(f, "{}", e.iter().map(|v| &v.0).join("."))
			}
		}
	}
}

#[derive(Debug, PartialEq)]
pub struct RsxIdentifier(pub String);

impl FromIterator<RsxIdentifier> for RsxIdentifier {
	fn from_iter<I: IntoIterator<Item = RsxIdentifier>>(iter: I) -> Self {
		RsxIdentifier(iter.into_iter().map(|v| v.0).join("-"))
	}
}
