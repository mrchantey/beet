use std::iter::FromIterator;

#[derive(Debug, PartialEq)]
pub struct RSChar(pub char);

#[derive(Debug, PartialEq)]
pub struct RSString(pub String);

impl FromIterator<char> for RSString {
	fn from_iter<I: IntoIterator<Item = char>>(iter: I) -> Self {
		RSString(iter.into_iter().collect())
	}
}
