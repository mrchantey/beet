#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(beet_core::test_runner))]
use beet_core::prelude::*;
use beet_rsx::prelude::*;

fn parse(bundle: impl Bundle) -> String {
	let mut world = World::new();
	world.init_resource::<Time>();
	world.spawn(bundle).get::<Name>().unwrap().to_string()
}

async fn parse_async(bundle: impl Bundle) -> String {
	let mut app = App::new();
	app.add_plugins(AsyncPlugin);
	app.world_mut().init_resource::<Time>();
	let entity = app.world_mut().spawn(bundle).id();
	let store = Store::new(None);
	app.add_observer(
		move |_: On<Insert, Name>,
		      query: Query<&Name>,
		      mut commands: Commands| {
			store.set(query.get(entity).unwrap().clone().xsome());
			commands.write_message(AppExit::Success);
		},
	);
	app.run_async().await;
	store.get().unwrap().to_string()
}

#[test]
fn simple() {
	#[construct]
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
fn take() {
	// some non-clone type
	struct Foo;

	#[construct(take)]
	fn Hello(name: String, foo: Foo) -> impl Bundle {
		let _ = foo;
		Name::new(name)
	}
	parse(Hello {
		name: "foo".into(),
		foo: Foo,
	})
	.xpect_eq("foo");
}
#[test]
fn system() {
	#[construct]
	fn Hello(name: String, time: Res<Time>) -> impl Bundle {
		let _ = time;
		Name::new(name)
	}
	parse(Hello { name: "foo".into() }).xpect_eq("foo");
}
#[beet_core::test]
async fn test_async() {
	#[construct]
	async fn Hello(name: String, my_entity: AsyncEntity) -> impl Bundle {
		let _ = my_entity;
		beet_core::exports::futures_lite::future::yield_now().await;
		Name::new(name)
	}

	parse_async(Hello { name: "foo".into() })
		.await
		.xpect_eq("foo");
}


#[test]
fn props() {
	#[construct]
	#[derive(Props)]
	fn Hello(
		#[field(default = "pizza".into())] name: String,
		time: Res<Time>,
	) -> impl Bundle {
		let _ = time;
		Name::new(name)
	}
	parse(Hello { name: "foo".into() }).xpect_eq("foo");
}
