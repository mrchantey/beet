pub struct ReactiveGraphRuntime;



#[cfg(test)]
mod test {
	use any_spawner::Executor;
	use reactive_graph::effect::Effect;
	use reactive_graph::owner::Owner;
	use reactive_graph::signal::RwSignal;
	use sweet::prelude::*;

	#[sweet::test]
	async fn works() {
		use std::sync::Arc;
		use std::sync::RwLock;

		#[cfg(target_arch = "wasm32")]
		let _ex = Executor::init_wasm_bindgen();
		#[cfg(not(target_arch = "wasm32"))]
		let _ex = Executor::init_tokio();

		let owner = Owner::new();
		owner.set();

		let block = async {
			let a = RwSignal::new(-1);

			// simulate an arbitrary side effect
			let b = Arc::new(RwLock::new(String::new()));

			// we forget it so it continues running
			// if it's dropped, it will stop listening
			Effect::new({
				let b = b.clone();
				move |_| {
					let formatted = format!("Value is {}", a());
					*b.write().unwrap() = formatted;
				}
			});

			Executor::tick().await;
			b.read().unwrap().as_str().xpect().to_be("Value is -1");

			a(1);

			Executor::tick().await;
			b.read().unwrap().as_str().xpect().to_be("Value is 1");
		};
		#[cfg(not(target_arch = "wasm32"))]
		tokio::task::LocalSet::new().run_until(block).await;

		#[cfg(target_arch = "wasm32")]
		block.await;
	}
}
