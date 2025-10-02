use beet_core::prelude::*;





#[derive(Deref, Reflect, Component)]
#[reflect(Component)]
#[relationship(relationship_target = ActionOf)]
pub struct Actions(pub Entity);

#[derive(Deref, Reflect, Component)]
#[reflect(Component)]
#[relationship_target(relationship = Actions, linked_spawn)]
pub struct ActionOf(Vec<Entity>);
