// use crate::prelude::*;
use bevy::prelude::*;


/// Signifies a behavior has started running.
#[derive(Debug, Default, Clone, Event, Reflect)]
#[reflect(Default)]
pub struct OnRun;
