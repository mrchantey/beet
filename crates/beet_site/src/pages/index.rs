use crate::prelude::*;
use beet::prelude::*;


const WEB_UI_DEMO: &str = r#"
#[template]
fn Counter(input: u32) -> impl Bundle {
	let (value, set_value) = signal(input);
	rsx! {
		<div>
			<button onclick={move |_| set_value(value() + 1)}>
				"Count: " {value}
			</button>
		</div>
	}
}
"#;

const SERVER_ACTIONS_DEMO: &str = r#"
// actions/add.rs
pub async fn get(input: JsonQuery<(i32, i32)>) -> Json<i32> {
	Json(input.0 + input.1)
}

// components/server_counter.rs
#[template]
pub fn ServerCounter(initial: i32) -> impl Bundle {
	let (get, set) = signal(initial);

	let onclick = move |_: Trigger<OnClick>| {
			spawn_local(async move {
				set(actions::add(get(), 1).await.unwrap());
			});
	};

	rsx! {
		<div>
			<Button
				variant=ButtonVariant::Outlined
				onclick=onclick>
				Server Cookie Count: {get}
			</Button>
		</div>
	}
}
"#;


const BEHAVIOR_DEMO: &str = r#"
#[template]
fn SayHello(name: String) -> impl Bundle {
	(
		Name::new("My Behavior"),
		Sequence,
		RunOnSpawn,
		children![
			(
				Name::new("Hello"),
				ReturnWith(RunResult::Success)
			),
			(
				Name::new(name),
				ReturnWith(RunResult::Success)
			)
		]
	)
}
"#;


pub fn get() -> impl Bundle {
	rsx! {
		<BeetContext>
			<ContentLayout>
				<BeetHeaderLinks slot="header-nav" />
				<div class="container">
				<h1>Beet</h1>
				<p><b>"Fullstack Bevy"</b></p>
				<Card style:cascade class="hero">
				<p>"Build applications that grow with Bevy ECS at every layer of the stack."
				<br/><br/>
				<span style="display: flex; align-items: center; justify-content: center;padding:0;">"🚧 Mind your step! 🚧"</span>				
				"Beet is under construction, basic development workflows are incomplete and untested. If this project is of interest please come and say hi in the"<a href="https://discord.com/channels/691052431525675048/1333204907414523964">bevy/beet discord channel</a>.</p>
					<footer>
						<Link
							style:cascade
							variant=ButtonVariant::Outlined
							href="https://github.com/mrchantey/beet"
							>Github</Link>
						<Link
							style:cascade
							variant=ButtonVariant::Primary
							href=routes::docs::index()
							>Get Started</Link>
					</footer>
				</Card>
				<h2>Very Bevy</h2>
				<ul>
				<li>"100% Open"<br/>"Beet inherits Bevy's MIT/Apache licenses"</li>
				<li>"100% Bevy"<br/>"Bevy primitives all the way down, including the beet cli!"</li>
				</ul>
				<h2>"Very Bevy Web UI"</h2>
					<ClientCounter client:load initial=1 />
				<Code style:cascade content=WEB_UI_DEMO/>
				<h2>"Very Bevy Server Actions"</h2>
				<p>"Pop open the dev tools to see your requests in flight!"</p>
				<ServerCounter client:load initial=1 />
				<Code style:cascade content=SERVER_ACTIONS_DEMO/>
				<h2>"Very Bevy Behavior"</h2>
				<Code style:cascade content=BEHAVIOR_DEMO/>
				</div>
			</ContentLayout>
		</BeetContext>
		<style>
		.container{
			display: flex;
			flex-direction: column;
			align-items: center;
			padding-top: 3.em;
			gap:1.em;
		}
		pre{
			max-width: 45.em;
			width: 45.em;
		}
		.hero{
			width: 30.em;
			max-width: 30.em;
		}
		.hero>footer{
			display: flex;
			flex-direction: row;
			gap: 1.em;
			justify-content: space-between;
			align-items: stretch;
		}
		.hero>footer>a{
			flex: 1;
		}
		.interactivity{
			display: flex;
			flex-direction: row;
			gap: 1.em;
		}
		</style>
	}
}
