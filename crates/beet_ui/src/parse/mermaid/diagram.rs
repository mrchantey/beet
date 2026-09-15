use beet_core::prelude::*;

/// A mermaid diagram awaiting its form: the fence body and the family its first
/// line names. Carried by the `<figure class="diagram">` the collector spawns
/// (or a `<Mermaid>` template), read by `materialize_diagrams`, which builds the
/// diagram's form beneath a figure that has no children yet.
#[derive(Debug, Clone, PartialEq, Eq, Reflect, Component)]
#[reflect(Component)]
pub struct MermaidDiagram {
	/// The mermaid source, the fence body verbatim.
	pub source: String,
	/// The family the first line names.
	pub kind: DiagramKind,
}

impl MermaidDiagram {
	/// A diagram from its source, its kind read from the first line.
	pub fn new(source: impl Into<String>) -> Self {
		let source = source.into();
		Self {
			kind: DiagramKind::parse(&source),
			source,
		}
	}

	/// The first meaningful line of the source (`graph LR`, `sequenceDiagram`),
	/// naming the diagram in a warning.
	pub fn title(&self) -> &str {
		DiagramKind::header(&self.source).unwrap_or("")
	}
}

/// The diagram family a mermaid source's first line names, the one fact the
/// `Auto` rule needs: a flowchart reflows as text, everything else is a picture
/// where the sink can show one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect)]
pub enum DiagramKind {
	/// `graph` or `flowchart` (any direction, including `flowchart-elk`).
	Flowchart,
	/// Every other type: sequence, class, state, ER, pie, gantt, ...
	Other,
}

impl DiagramKind {
	/// The family named by the first non-blank line that is not a `%%` comment
	/// or `%%{init}%%` directive.
	pub fn parse(source: &str) -> Self {
		match Self::header(source)
			.and_then(|line| line.split_whitespace().next())
			.map(|word| word.to_ascii_lowercase())
			.as_deref()
		{
			Some("graph" | "flowchart" | "flowchart-elk") => Self::Flowchart,
			_ => Self::Other,
		}
	}

	/// The first non-blank, non-comment line of `source`.
	fn header(source: &str) -> Option<&str> {
		source
			.lines()
			.map(str::trim)
			.find(|line| !line.is_empty() && !line.starts_with("%%"))
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[beet_core::test]
	fn parses_kind_from_the_header() {
		DiagramKind::parse("graph LR\nA-->B").xpect_eq(DiagramKind::Flowchart);
		DiagramKind::parse("%%{init: {}}%%\n\n  Flowchart TD\nA-->B")
			.xpect_eq(DiagramKind::Flowchart);
		DiagramKind::parse("sequenceDiagram\nA->>B: hi")
			.xpect_eq(DiagramKind::Other);
		DiagramKind::parse("").xpect_eq(DiagramKind::Other);
		MermaidDiagram::new("%% note\npie title Pets")
			.title()
			.xpect_eq("pie title Pets");
	}
}
