#![allow(missing_docs)]
// use crate::prelude::*;
use beet_core::prelude::*;


/// Store and load scenes as needed
#[derive(Clone, Component, Reflect)]
#[reflect(Component)]
pub struct SceneStore {
	scene: StoredScene,
}


#[derive(Clone, Reflect)]
pub enum StoredScene {
	Inline(String),
	Blob,
}

impl SceneStore {}
