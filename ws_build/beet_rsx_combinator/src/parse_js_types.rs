use std::iter::FromIterator;

#[derive(Debug, PartialEq)]
pub struct JSBool(pub bool);

#[derive(Debug, PartialEq)]
pub struct JSNumber(pub f64);

#[derive(Debug, PartialEq)]
pub struct JSSingleStringCharacter(pub char);

#[derive(Debug, PartialEq)]
pub struct JSSingleStringCharacters(pub String);

impl FromIterator<JSSingleStringCharacter> for JSSingleStringCharacters {
	fn from_iter<I: IntoIterator<Item = JSSingleStringCharacter>>(
		iter: I,
	) -> Self {
		JSSingleStringCharacters(iter.into_iter().map(|c| c.0).collect())
	}
}

#[derive(Debug, PartialEq)]
pub struct JSDoubleStringCharacter(pub char);

#[derive(Debug, PartialEq)]
pub struct JSDoubleStringCharacters(pub String);

impl FromIterator<JSDoubleStringCharacter> for JSDoubleStringCharacters {
	fn from_iter<I: IntoIterator<Item = JSDoubleStringCharacter>>(
		iter: I,
	) -> Self {
		JSDoubleStringCharacters(iter.into_iter().map(|c| c.0).collect())
	}
}

#[derive(Debug, PartialEq)]
pub struct JSIdentifierStart(pub char);

#[derive(Debug, PartialEq)]
pub struct JSIdentifierPart(pub String);

impl FromIterator<char> for JSIdentifierPart {
	fn from_iter<I: IntoIterator<Item = char>>(iter: I) -> Self {
		JSIdentifierPart(iter.into_iter().collect())
	}
}
