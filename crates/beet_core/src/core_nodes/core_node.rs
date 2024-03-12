use super::*;
use beet_ecs::action_list;
use beet_ecs::prelude::*;


// for now we need to manually keep in sync with crates/beet_ecs/src/builtin_nodes/builtin_nodes.rs
action_list!(CoreNode, [
	//core
	Translate,
	//steer
	Seek,
	Wander,
	SetVelocity,
	FindSteerTarget,
	DespawnSteerTarget,
	ScoreSteerTarget,
	SucceedOnArrive,
	//ecs
	EcsNode
]);
