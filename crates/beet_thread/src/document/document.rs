use async_lock::RwLock;
// use crate::prelude::*;
use beet_core::prelude::*;
use serde::Deserialize;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;
use std::sync::Arc;
use uuid::Uuid;


pub trait Document: 'static + Send + Sync + Sized {
	type Id: DocId;
	fn id(&self) -> Self::Id;
}
pub trait DocId:
	// 'static
	Send
	+ Sync
	+ Debug
	+ Clone
	+ Hash
	+ PartialEq
	+ Eq
	+ Serialize
	+ DeserializeOwned
{
	/// Convenience for usage like hashmap keys
	fn into_bytes(&self) -> Vec<u8>;
	/// Convenience for usage like URL paths, etc,
	/// by default converting the bytes to base64
	fn into_string(&self) -> String {
			use base64::Engine;
			base64::engine::general_purpose::STANDARD.encode(self.into_bytes())
	}
}

impl<T1: DocId, T2: DocId> DocId for (T1, T2) {
	fn into_bytes(&self) -> Vec<u8> {
		let mut bytes = Vec::new();
		bytes.extend_from_slice(&self.0.into_bytes());
		bytes.extend_from_slice(&self.1.into_bytes());
		bytes
	}
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

impl DocId for String {
	fn into_bytes(&self) -> Vec<u8> { self.as_bytes().to_vec() }
	fn into_string(&self) -> String { self.clone() }
}

impl<M> DocId for Uuid7<M> {
	fn into_bytes(&self) -> Vec<u8> { self.uuid_v7.as_bytes().to_vec() }
	fn into_string(&self) -> String { self.uuid_v7.to_string() }
}

unsafe impl<M> Send for Uuid7<M> {}
unsafe impl<M> Sync for Uuid7<M> {}


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

pub trait DocStore<T: Document> {
	#[track_caller]
	fn insert(&self, value: T) -> BoxedFuture<'_, Result<T::Id>>;
	#[track_caller]
	fn get(&self, id: T::Id) -> BoxedFuture<'_, Result<T>>
	where
		T: Clone;
}

#[derive(Debug, Default, Clone)]
pub struct ArcDocMap<T: Document>(Arc<RwLock<DocMap<T>>>);
impl<T: Document> ArcDocMap<T> {
	pub fn new(map: DocMap<T>) -> Self { Self(Arc::new(RwLock::new(map))) }
}

impl<T> DocStore<T> for ArcDocMap<T>
where
	T: Document,
{
	fn insert(&self, value: T) -> BoxedFuture<'_, Result<T::Id>> {
		// let map = self.0.clone();
		Box::pin(async move {
			let mut map = self.0.write().await;
			Ok(map.insert(value))
		})
	}
	fn get(&self, id: T::Id) -> BoxedFuture<'_, Result<T>>
	where
		T: Clone,
	{
		Box::pin(async move { self.0.read().await.get(id).cloned() })
	}
}


/// In-memory collection of documents, mapped by their id.
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
	#[track_caller]
	pub fn get(&self, id: T::Id) -> Result<&T> {
		self.0.get(&id).ok_or_else(
			#[cfg_attr(feature = "nightly", track_caller)]
			|| bevyhow!("Id {id:?} not found in DocMap"),
		)
	}
	#[track_caller]
	pub fn get_mut(&mut self, id: T::Id) -> Result<&mut T> {
		self.0.get_mut(&id).ok_or_else(
			#[cfg_attr(feature = "nightly", track_caller)]
			|| bevyhow!("Id {id:?} not found in DocMap"),
		)
	}
	pub fn remove(&mut self, id: T::Id) -> Option<T> { self.0.remove(&id) }
	pub fn drain(&mut self) -> impl Iterator<Item = T> {
		self.0.drain().map(|(_, v)| v)
	}
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
