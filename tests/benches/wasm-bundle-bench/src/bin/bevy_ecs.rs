use bevy_ecs::{prelude::*, schedule::ScheduleLabel};
use web_sys::*;

fn main() {
    console_error_panic_hook::set_once();
    Schedule::new(Run)
        .add_systems(|| {
            console::log_1(&"Hello bevy_ecs".into());
        })
        .run(&mut World::default());
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, ScheduleLabel)]
struct Run;
