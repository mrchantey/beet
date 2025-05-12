use any_spawner::Executor;
use reactive_graph::effect::Effect;
use reactive_graph::owner::Owner;
use reactive_graph::signal::signal;
use web_sys::*;

fn main() {
    console_error_panic_hook::set_once();
    wasm_bindgen_futures::spawn_local(run());
}

async fn run() {
    Executor::init_wasm_bindgen().unwrap();

    let owner = Owner::new();
    owner.set();

    let (get1, set1) = signal(0);
    let (get2, set2) = signal(0);

    let _effect = Effect::new(Box::new(move |_| {
        set2(get1() + 1);
    }));

    set1(4);

    assert_eq!(get1(), 4);

    Executor::tick().await;

    assert_eq!(get2(), 5);

    console::log_1(&"Hello reactive_graph".into());
}
