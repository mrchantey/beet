#[template]
fn SayHello(name: String) -> impl Bundle {
	(
		Name::new("My Behavior"),
		Sequence,
		RunOnSpawn,
		children![
			(
				Name::new("Hello"),
				EndOnRun(SUCCESS)
			),
			(
				Name::new(name),
				EndOnRun(SUCCESS)
			)
		]
	)
}