use crate::prelude::*;
use std::iter::FromIterator;

#[derive(Default, Debug, PartialEq)]
pub struct RsxChildren(pub Box<[RsxChild]>);

impl From<Option<RsxChildren>> for RsxChildren {
	fn from(children: Option<RsxChildren>) -> Self {
		children.unwrap_or_default()
	}
}

impl From<Vec<RsxChild>> for RsxChildren {
	fn from(vec: Vec<RsxChild>) -> Self { RsxChildren(vec.into_boxed_slice()) }
}

impl FromIterator<RsxChild> for RsxChildren {
	fn from_iter<I: IntoIterator<Item = RsxChild>>(iter: I) -> Self {
		RsxChildren::from(iter.into_iter().collect::<Vec<_>>())
	}
}

#[derive(Debug, PartialEq)]
pub enum RsxChild {
	Element(RsxElement),
	Text(RsxText),
	CodeBlock(RsxParsedExpression),
}

#[derive(Debug, PartialEq)]
pub struct RsxText(pub String);

impl FromIterator<RsxTextCharacter> for RsxText {
	fn from_iter<I: IntoIterator<Item = RsxTextCharacter>>(iter: I) -> Self {
		RsxText(iter.into_iter().map(|c| c.0).collect())
	}
}

#[derive(Debug, PartialEq)]
pub struct RsxTextCharacter(pub char);
