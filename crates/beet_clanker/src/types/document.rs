// use crate::prelude::*;
use beet_core::prelude::*;
use serde::Deserialize;
use serde::Serialize;
use std::marker::PhantomData;
use uuid::Uuid;


pub trait Document: Sized {
	fn id(&self) -> DocId<Self>;
}

#[derive(Debug, Serialize, Deserialize, Component)]
pub struct DocId<M = ()> {
	uuid_v7: Uuid,
	phantom_data: PhantomData<M>,
}

impl<M> DocId<M> {
	pub fn new_now() -> Self {
		Self {
			uuid_v7: Uuid::now_v7(),
			phantom_data: default(),
		}
	}
	pub fn uuid(&self) -> Uuid { self.uuid_v7 }
}

impl<M> Default for DocId<M> {
	fn default() -> Self { Self::new_now() }
}


impl<M> Copy for DocId<M> {}
impl<M> Clone for DocId<M> {
	fn clone(&self) -> Self { *self }
}
impl<M> PartialEq for DocId<M> {
	fn eq(&self, other: &Self) -> bool { self.uuid_v7 == other.uuid_v7 }
}

impl<M> Eq for DocId<M> {}

impl<M> PartialOrd for DocId<M> {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		self.uuid_v7.partial_cmp(&other.uuid_v7)
	}
}

impl<M> Ord for DocId<M> {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		self.uuid_v7.cmp(&other.uuid_v7)
	}
}

impl<M> std::hash::Hash for DocId<M> {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.uuid_v7.hash(state)
	}
}



impl<M> std::ops::Deref for DocId<M> {
	type Target = Uuid;
	fn deref(&self) -> &Self::Target { &self.uuid_v7 }
}


impl<M> std::fmt::Display for DocId<M> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.uuid_v7)
	}
}


#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocMap<T>(HashMap<DocId<T>, T>);

impl<T> Default for DocMap<T> {
	fn default() -> Self { Self::new() }
}

impl<T> DocMap<T> {
	pub fn new() -> Self { Self(HashMap::new()) }
	pub fn contains_key(&self, id: DocId<T>) -> bool {
		self.0.contains_key(&id)
	}
	pub fn insert(&mut self, doc: T) -> DocId<T>
	where
		T: Document,
	{
		let id = doc.id();
		self.0.insert(id, doc);
		id
	}
	pub fn get(&self, id: DocId<T>) -> Result<&T> {
		self.0
			.get(&id)
			.ok_or_else(|| bevyhow!("DocId {id} not found in DocMap"))
	}
	pub fn get_mut(&mut self, id: DocId<T>) -> Result<&mut T> {
		self.0
			.get_mut(&id)
			.ok_or_else(|| bevyhow!("DocId {id} not found in DocMap"))
	}
	pub fn remove(&mut self, id: DocId<T>) -> Option<T> { self.0.remove(&id) }
	pub fn iter(&self) -> impl Iterator<Item = (&DocId<T>, &T)> {
		self.0.iter()
	}
	pub fn iter_mut(&mut self) -> impl Iterator<Item = (&DocId<T>, &mut T)> {
		self.0.iter_mut()
	}
	pub fn keys(&self) -> impl Iterator<Item = &DocId<T>> { self.0.keys() }
	pub fn values(&self) -> impl Iterator<Item = &T> { self.0.values() }
	pub fn values_mut(&mut self) -> impl Iterator<Item = &mut T> {
		self.0.values_mut()
	}
}
