pub struct ReactiveGraphRuntime;



#[cfg(test)]
mod test {
	use any_spawner::Executor;
	use reactive_graph::effect::RenderEffect;
	use reactive_graph::owner::Owner;
	use reactive_graph::signal::signal;
	use std::sync::Arc;
	use std::sync::Mutex;
	use sweet::prelude::*;

	#[sweet::test]
	#[ignore = "how to reactive graph?"]
	async fn works() {
		Executor::init_tokio().unwrap();
		let owner = Owner::new();
		owner.set();

		reactive_graph::spawn(async {
			let (get, set) = signal(7);

			let val = Arc::new(Mutex::new(0));
			let val2 = val.clone();
			let _effect = RenderEffect::new(Box::new(move |_| {
				*val2.lock().unwrap() = get();
			}));

			set(4);
			expect(&*val.lock().unwrap()).to_be(&4);
		});
	}
}
