use bevy_derive::Deref;
use bevy_derive::DerefMut;
use core::fmt;
use serde::Deserialize;
use serde::Serialize;
use std::fmt::Display;


pub type TopicKey = u64;

#[derive(
	Debug,
	Clone,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	// Deref,
	// DerefMut,
	Serialize,
	Deserialize,
)]
pub struct TopicPath {
	path: String,
	key: Option<TopicKey>,
}
impl TopicPath {
	///
	pub fn from_str(path: &str) -> Self {
		if path.contains("/") {
			log::warn!("Topic Path contains '/', this is not allowed: {}", path);
		}
		let mut split = path.split(":");
		let path = split.next().unwrap_or_default();
		let key = split.next().and_then(|s| {
			if let Ok(parsed) = s.parse() {
				Some(parsed)
			} else {
				log::warn!("Topic Path failed to parse key: {}", s);
				None
			}
		});
		Self {
			path: path.to_string(),
			key,
		}
	}
}
impl fmt::Display for TopicPath {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		if let Some(key) = self.key {
			write!(f, "{}:{}", self.path, key)
		} else {
			write!(f, "{}", self.path)
		}
	}
}


impl Into<TopicPath> for String {
	fn into(self) -> TopicPath { TopicPath::from_str(&self) }
}
impl Into<TopicPath> for &str {
	fn into(self) -> TopicPath { TopicPath::from_str(self) }
}

#[derive(
	Debug,
	Clone,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Deref,
	DerefMut,
	Serialize,
	Deserialize,
)]
pub struct TopicAddress(pub Vec<TopicPath>);

impl Display for TopicAddress {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let mut iter = self.iter();
		if let Some(path) = iter.next() {
			write!(f, "{path}")?;
		}
		for path in iter {
			write!(f, "/{path}")?;
		}
		Ok(())
	}
}

impl Into<TopicAddress> for String {
	fn into(self) -> TopicAddress { TopicAddress::from_str(&self) }
}
impl Into<TopicAddress> for &str {
	fn into(self) -> TopicAddress { TopicAddress::from_str(self) }
}

impl Into<TopicAddress> for &TopicAddress {
	fn into(self) -> TopicAddress { self.clone() }
}

impl TopicAddress {
	pub fn from_str(addr: &str) -> Self {
		let paths = addr.split("/").map(|s| s.into()).collect();
		Self(paths)
	}
	pub fn new(addr: impl Into<TopicAddress>) -> Self { addr.into() }

	pub fn push(&mut self, path: impl Into<TopicPath>) {
		self.0.push(path.into());
	}

	pub fn pop(&mut self) -> Option<TopicPath> { self.0.pop() }
}
