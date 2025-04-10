use crate::prelude::*;

// JS Types

impl From<bool> for JSBool {
	fn from(v: bool) -> Self { JSBool(v) }
}

impl From<f64> for JSNumber {
	fn from(v: f64) -> Self { JSNumber(v) }
}

impl From<char> for JSSingleStringCharacter {
	fn from(v: char) -> Self { JSSingleStringCharacter(v) }
}

impl From<&'static str> for JSSingleStringCharacters {
	fn from(v: &'static str) -> Self { JSSingleStringCharacters(v.into()) }
}

impl From<char> for JSDoubleStringCharacter {
	fn from(v: char) -> Self { JSDoubleStringCharacter(v) }
}

impl From<&'static str> for JSDoubleStringCharacters {
	fn from(v: &'static str) -> Self { JSDoubleStringCharacters(v.into()) }
}

impl From<char> for JSIdentifierStart {
	fn from(v: char) -> Self { JSIdentifierStart(v) }
}

impl From<&'static str> for JSIdentifierPart {
	fn from(v: &'static str) -> Self { JSIdentifierPart(v.into()) }
}

// Rust types

impl From<&'static str> for RSString {
	fn from(v: &'static str) -> Self { RSString(v.into()) }
}

// RSX Elements

impl From<&'static str> for RsxSelfClosingElement {
	fn from(n: &'static str) -> Self {
		RsxSelfClosingElement(n.into(), RsxAttributes::from(vec![]))
	}
}

impl From<(&'static str, &'static str)> for RsxSelfClosingElement {
	fn from((ns, n): (&'static str, &'static str)) -> Self {
		RsxSelfClosingElement((ns, n).into(), RsxAttributes::from(vec![]))
	}
}

impl<'a> From<&'a [&'static str]> for RsxSelfClosingElement {
	fn from(xs: &'a [&'static str]) -> Self {
		RsxSelfClosingElement(xs.into(), RsxAttributes::from(vec![]))
	}
}

impl From<&'static str> for RsxNormalElement {
	fn from(n: &'static str) -> Self {
		RsxNormalElement(
			n.into(),
			RsxAttributes::from(vec![]),
			RsxChildren::from(vec![]),
		)
	}
}

impl From<(&'static str, &'static str)> for RsxNormalElement {
	fn from((ns, n): (&'static str, &'static str)) -> Self {
		RsxNormalElement(
			(ns, n).into(),
			RsxAttributes::from(vec![]),
			RsxChildren::from(vec![]),
		)
	}
}

impl<'a> From<&'a [&'static str]> for RsxNormalElement {
	fn from(xs: &'a [&'static str]) -> Self {
		RsxNormalElement(
			xs.into(),
			RsxAttributes::from(vec![]),
			RsxChildren::from(vec![]),
		)
	}
}

impl From<&'static str> for RsxOpeningElement {
	fn from(n: &'static str) -> Self {
		RsxOpeningElement(n.into(), RsxAttributes::from(vec![]))
	}
}

impl From<(&'static str, &'static str)> for RsxOpeningElement {
	fn from((ns, n): (&'static str, &'static str)) -> Self {
		RsxOpeningElement((ns, n).into(), RsxAttributes::from(vec![]))
	}
}

impl<'a> From<&'a [&'static str]> for RsxOpeningElement {
	fn from(xs: &'a [&'static str]) -> Self {
		RsxOpeningElement(xs.into(), RsxAttributes::from(vec![]))
	}
}

impl From<&'static str> for RsxElementName {
	fn from(n: &'static str) -> Self { RsxElementName::Name(n.into()) }
}

impl From<(&'static str, &'static str)> for RsxElementName {
	fn from((ns, n): (&'static str, &'static str)) -> Self {
		RsxElementName::NamedspacedName(ns.into(), n.into())
	}
}

impl<'a> From<&'a [&'static str]> for RsxElementName {
	fn from(xs: &'a [&'static str]) -> Self {
		let vec = xs.into_iter().map(|v| (*v).into()).collect::<Vec<_>>();
		RsxElementName::MemberExpression(vec.into())
	}
}

impl From<&'static str> for RsxIdentifier {
	fn from(v: &'static str) -> Self { RsxIdentifier(v.into()) }
}

// RSX Attributes

impl From<(&'static str, &'static str)> for RsxAttribute {
	fn from((k, v): (&'static str, &'static str)) -> Self {
		RsxAttribute::Named(k.into(), v.into())
	}
}

impl From<&'static str> for RsxAttributeName {
	fn from(n: &'static str) -> Self { RsxAttributeName::Name(n.into()) }
}

impl From<bool> for RsxAttributeValue {
	fn from(v: bool) -> Self { RsxAttributeValue::Boolean(v.into()) }
}

impl From<f64> for RsxAttributeValue {
	fn from(v: f64) -> Self { RsxAttributeValue::Number(v.into()) }
}

impl From<&'static str> for RsxAttributeValue {
	fn from(v: &'static str) -> Self {
		if v == "true" {
			RsxAttributeValue::Default
		} else {
			RsxAttributeValue::Str(v.into())
		}
	}
}

impl From<(&'static str, char)> for RsxAttributeValue {
	fn from((v, t): (&'static str, char)) -> Self {
		if v == "true" {
			RsxAttributeValue::Default
		} else {
			RsxAttributeValue::Str((v, t).into())
		}
	}
}

impl From<bool> for RsxAttributeBoolean {
	fn from(v: bool) -> Self { RsxAttributeBoolean(v) }
}

impl From<f64> for RsxAttributeNumber {
	fn from(v: f64) -> Self { RsxAttributeNumber(v) }
}

impl From<&'static str> for RsxAttributeString {
	fn from(v: &'static str) -> Self {
		RsxAttributeString::SingleQuoted(v.into())
	}
}

impl From<(&'static str, char)> for RsxAttributeString {
	fn from((n, t): (&'static str, char)) -> Self {
		match t {
			'"' => RsxAttributeString::SingleQuoted(n.into()),
			'\'' => RsxAttributeString::DoubleQuoted(n.into()),
			_ => panic!("Unsupported string format"),
		}
	}
}

// RSX Children

impl From<&'static str> for RsxChild {
	fn from(v: &'static str) -> Self { RsxChild::Text(v.into()) }
}

impl From<&'static str> for RsxText {
	fn from(v: &'static str) -> Self { RsxText(v.into()) }
}

impl From<char> for RsxTextCharacter {
	fn from(v: char) -> Self { RsxTextCharacter(v) }
}
