// use crate::prelude::*;
use beet_core::prelude::*;
use serde::Deserialize;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;
use uuid::Uuid;


pub trait Document: Sized {
	type Id: DocId;
	fn id(&self) -> Self::Id;
}

#[derive(Serialize, Deserialize, Component)]
pub struct Uuid7<M = ()> {
	uuid_v7: Uuid,
	#[serde(skip)]
	phantom_data: PhantomData<M>,
}

impl<M> std::fmt::Debug for Uuid7<M> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.uuid_v7)
	}
}

impl<M> DocId for Uuid7<M> {
	fn into_bytes(&self) -> Vec<u8> { self.uuid_v7.as_bytes().to_vec() }
}
impl<M1, M2> DocId for (Uuid7<M1>, Uuid7<M2>) {
	fn into_bytes(&self) -> Vec<u8> {
		let mut bytes = Vec::with_capacity(32);
		bytes.extend_from_slice(self.0.uuid_v7.as_bytes());
		bytes.extend_from_slice(self.1.uuid_v7.as_bytes());
		bytes
	}
}

pub trait DocId:
	Debug + Clone + Hash + PartialEq + Eq + Serialize + DeserializeOwned
{
	/// Convenience for usage like hashmap keys
	fn into_bytes(&self) -> Vec<u8>;
}


impl<M> Uuid7<M> {
	pub fn new_now() -> Self {
		Self {
			uuid_v7: Uuid::now_v7(),
			phantom_data: default(),
		}
	}
	pub fn uuid(&self) -> Uuid { self.uuid_v7 }
}

impl<M> Default for Uuid7<M> {
	fn default() -> Self { Self::new_now() }
}


impl<M> Copy for Uuid7<M> {}
impl<M> Clone for Uuid7<M> {
	fn clone(&self) -> Self { *self }
}
impl<M> PartialEq for Uuid7<M> {
	fn eq(&self, other: &Self) -> bool { self.uuid_v7 == other.uuid_v7 }
}

impl<M> Eq for Uuid7<M> {}

impl<M> PartialOrd for Uuid7<M> {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		self.uuid_v7.partial_cmp(&other.uuid_v7)
	}
}

impl<M> Ord for Uuid7<M> {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		self.uuid_v7.cmp(&other.uuid_v7)
	}
}

impl<M> std::hash::Hash for Uuid7<M> {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.uuid_v7.hash(state)
	}
}



impl<M> std::ops::Deref for Uuid7<M> {
	type Target = Uuid;
	fn deref(&self) -> &Self::Target { &self.uuid_v7 }
}


impl<M> std::fmt::Display for Uuid7<M> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.uuid_v7)
	}
}


#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocMap<T: Document>(HashMap<T::Id, T>);

impl<T: Document> Default for DocMap<T> {
	fn default() -> Self { Self::new() }
}

impl<T: Document> DocMap<T> {
	pub fn new() -> Self { Self(HashMap::new()) }
	pub fn contains_key(&self, id: T::Id) -> bool { self.0.contains_key(&id) }
	pub fn insert(&mut self, doc: T) -> T::Id
	where
		T: Document,
	{
		let id = doc.id();
		self.0.insert(id.clone(), doc);
		id
	}
	pub fn get(&self, id: T::Id) -> Result<&T> {
		self.0
			.get(&id)
			.ok_or_else(|| bevyhow!("Id {id:?} not found in DocMap"))
	}
	pub fn get_mut(&mut self, id: T::Id) -> Result<&mut T> {
		self.0
			.get_mut(&id)
			.ok_or_else(|| bevyhow!("Id {id:?} not found in DocMap"))
	}
	pub fn remove(&mut self, id: T::Id) -> Option<T> { self.0.remove(&id) }
	pub fn iter(&self) -> impl Iterator<Item = (&T::Id, &T)> { self.0.iter() }
	pub fn iter_mut(&mut self) -> impl Iterator<Item = (&T::Id, &mut T)> {
		self.0.iter_mut()
	}
	pub fn keys(&self) -> impl Iterator<Item = &T::Id> { self.0.keys() }
	pub fn values(&self) -> impl Iterator<Item = &T> { self.0.values() }
	pub fn values_mut(&mut self) -> impl Iterator<Item = &mut T> {
		self.0.values_mut()
	}
}
