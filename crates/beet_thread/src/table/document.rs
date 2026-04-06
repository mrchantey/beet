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


pub trait Table: 'static + Send + Sync + Sized {
	type Id: TableId;
	fn id(&self) -> Self::Id;
}

/// The primary key for a table
pub trait TableId:
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

impl<T1: TableId, T2: TableId> TableId for (T1, T2) {
	fn into_bytes(&self) -> Vec<u8> {
		let mut bytes = Vec::new();
		bytes.extend_from_slice(&self.0.into_bytes());
		bytes.extend_from_slice(&self.1.into_bytes());
		bytes
	}
}

#[derive(Serialize, Deserialize, Reflect, Component)]
#[reflect(opaque)]
#[reflect(Serialize, Deserialize)]
pub struct Uuid7<M: 'static = ()> {
	uuid_v7: Uuid,
	#[serde(skip)]
	#[reflect(ignore)]
	phantom_data: PhantomData<M>,
}

impl<M> std::fmt::Debug for Uuid7<M> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.uuid_v7)
	}
}

impl TableId for String {
	fn into_bytes(&self) -> Vec<u8> { self.as_bytes().to_vec() }
	fn into_string(&self) -> String { self.clone() }
}

impl<M> TableId for Uuid7<M> {
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

pub trait TableStore<T: Table> {
	#[track_caller]
	fn insert(&self, value: T) -> BoxedFuture<'_, Result<T::Id>>;
	#[track_caller]
	fn get(&self, id: T::Id) -> BoxedFuture<'_, Result<T>>
	where
		T: Clone;
}

#[derive(Debug, Default, Clone)]
pub struct ArcTableMap<T: Table>(Arc<RwLock<TableMap<T>>>);
impl<T: Table> ArcTableMap<T> {
	pub fn new(map: TableMap<T>) -> Self { Self(Arc::new(RwLock::new(map))) }
}

impl<T> TableStore<T> for ArcTableMap<T>
where
	T: Table,
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


/// In-memory collection of tables, mapped by their id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableMap<T: Table>(HashMap<T::Id, T>);

impl<T: Table> Default for TableMap<T> {
	fn default() -> Self { Self::new() }
}

impl<T: Table> TableMap<T> {
	pub fn new() -> Self { Self(HashMap::new()) }
	pub fn contains_key(&self, id: T::Id) -> bool { self.0.contains_key(&id) }
	pub fn insert(&mut self, table: T) -> T::Id
	where
		T: Table,
	{
		let id = table.id();
		self.0.insert(id.clone(), table);
		id
	}
	#[track_caller]
	pub fn get(&self, id: T::Id) -> Result<&T> {
		self.0.get(&id).ok_or_else(
			#[cfg_attr(feature = "nightly", track_caller)]
			|| bevyhow!("Id {id:?} not found in TableMap"),
		)
	}
	#[track_caller]
	pub fn get_mut(&mut self, id: T::Id) -> Result<&mut T> {
		self.0.get_mut(&id).ok_or_else(
			#[cfg_attr(feature = "nightly", track_caller)]
			|| bevyhow!("Id {id:?} not found in TableMap"),
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
