pub struct ReactiveGraphRuntime;



#[cfg(test)]
mod test {
	use any_spawner::Executor;
	use reactive_graph::owner::Owner;
	use reactive_graph::signal::signal;

	#[sweet::test]
	async fn works() {
		use reactive_graph::effect::Effect;


		#[cfg(not(target_arch = "wasm32"))]
		Executor::init_tokio().unwrap();

		#[cfg(target_arch = "wasm32")]
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

		println!("done");
	}
}
