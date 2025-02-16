use crate::prelude::*;
use bevy::prelude::Deref;
use bevy::prelude::DerefMut;
use bevy::prelude::*;
use bevy::utils::HashMap;

/// Marker to identify an rsx node for O(1) lookup
/// using SparseSet to avoid table fragmentation.
#[derive(Debug, Default, Component, Reflect)]
#[reflect(Default, Component)]
#[component(storage = "SparseSet")]
pub struct RsxIdxMarker<const IDX: u32>;
/// Marker to identify an rsx node for O(1) lookup
/// using SparseSet to avoid table fragmentation.
#[derive(Debug, Default, Resource, Reflect, Deref, DerefMut)]
#[reflect(Default, Resource)]
pub struct RsxIdxMap(pub HashMap<RsxIdx, Entity>);
