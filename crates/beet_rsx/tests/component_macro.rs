#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]

use beet_core::prelude::*;
use beet_rsx::prelude::*;
use sweet::prelude::*;

fn parse(bundle: impl Bundle) -> String {
	let mut world = World::new();
	world.init_resource::<Time>();
	world.spawn(bundle).get::<Name>().unwrap().to_string()
}

async fn parse_async(bundle: impl Bundle) -> String {
	let mut world = AsyncPlugin::world();
	world.init_resource::<Time>();
	let entity = world.spawn(bundle).id();
	AsyncRunner::flush_async_tasks(&mut world).await;
	world.entity(entity).get::<Name>().unwrap().to_string()
}

#[test]
fn simple() {
	#[component]
	fn Hello(name: String, my_entity: Entity, r#type: String) -> impl Bundle {
		let _ = my_entity;
		let _ = r#type;
		Name::new(name)
	}
	parse(Hello {
		name: "foo".into(),
		r#type: "bar".into(),
	})
	.xpect_eq("foo");
}
#[test]
fn system() {
	#[component]
	fn Hello(name: String, time: Res<Time>) -> impl Bundle {
		let _ = time;
		Name::new(name)
	}
	parse(Hello { name: "foo".into() }).xpect_eq("foo");
}
#[sweet::test]
async fn test_async() {
	#[component]
	async fn Hello(name: String, my_entity: AsyncEntity) -> impl Bundle {
		let _ = my_entity;
		beet_core::exports::futures_lite::future::yield_now().await;
		Name::new(name)
	}

	parse_async(Hello { name: "foo".into() })
		.await
		.xpect_eq("foo");
}
