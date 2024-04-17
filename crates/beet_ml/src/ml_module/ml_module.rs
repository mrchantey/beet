use crate::prelude::*;
use beet_ecs::prelude::*;
use bevy::prelude::*;

#[derive(Debug, Clone, BeetModule)]
#[actions(SentenceScorer)]
#[components(Sentence)]
pub struct MlModule;
