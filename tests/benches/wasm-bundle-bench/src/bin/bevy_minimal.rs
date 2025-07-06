use bevy::prelude::*;
use web_sys::*;

fn main() {
    console_error_panic_hook::set_once();
    App::new()
        .add_plugins(MinimalPlugins)
        .add_systems(Startup, || {
            console::log_1(&"Hello bevy_minimal".into());
        })
        .run();
}
