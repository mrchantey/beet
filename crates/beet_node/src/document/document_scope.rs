use beet_core::prelude::*;

/// Marker component for entities that serve as document scope boundaries.
///
/// When a [`DocumentPath::Ancestor`] is used, the system walks up the entity
/// hierarchy looking for the nearest ancestor with this marker.
/// In `beet_router`, scene components like `CallerScene` and `FileScene` require `DocumentScope`.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component)]
pub struct DocumentScope;
