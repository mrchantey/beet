use crate::prelude::*;
use bevy::ecs::component::Mutable;
use bevy::prelude::*;
use bevy::reflect::Reflectable;
use flume::Receiver;
use flume::Sender;

pub trait SignalPayload: 'static + Send + Sync + Clone + Reflectable {}
impl<T: 'static + Send + Sync + Clone + Reflectable> SignalPayload for T {}



pub fn observer(){


}