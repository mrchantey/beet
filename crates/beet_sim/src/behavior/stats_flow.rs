use beet_flow::prelude::*;
use bevy::prelude::*;


#[derive(Debug, Default, Clone, PartialEq, Component, Reflect, Action)]
#[systems(stats_flow.in_set(TickSet))]
#[reflect(Default, Component)]
pub struct StatsFlow {

}

fn stats_flow(query: Query<&StatsFlow, With<Running>>) {

	for stats_flow in query.iter() {
		
	}
}
