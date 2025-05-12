use bevy::prelude::*;
use web_sys::*;

fn main() {
    console_error_panic_hook::set_once();
    // unsafe {
    //     Instant::set_elapsed(|| std::time::Duration::from_millis(0));
    // }
    // console::log_1(&"Hello pizza".into());
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, || {
            console::log_1(&"Hello bevy_default".into());
        })
        .run();
}
