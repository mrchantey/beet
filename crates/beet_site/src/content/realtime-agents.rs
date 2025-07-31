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