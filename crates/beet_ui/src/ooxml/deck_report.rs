//! The triage of a deck, appended to its tree after the parse.
use super::deck::*;
use beet_core::prelude::*;

/// Marks a deck whose triage is appended, so a later run of
/// [`PostParseTree`](crate::prelude::PostParseTree) leaves it be.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Reflect, Component)]
#[reflect(Component, Default)]
pub struct DeckReported;

/// The triage a reader of a deck reads before transcribing it, appended to
/// the deck's tree as projection only: a header with the deck's size, each
/// slide's picture inventory and a note where it has no text, and a closing
/// report sorting every slide by what carries it, words or a picture, and
/// naming hidden slides, deleted slides' leftovers, recurring pictures,
/// same-size pictures and exact duplicates.
pub(crate) struct DeckReport;

/// One slide as the report reads it.
struct SlideReport {
	section: Entity,
	slide: Slide,
	/// Each picture: its source, size and cover.
	pictures: Vec<(SmolStr, SlidePicture)>,
	duplicate_of: Option<u32>,
}

impl DeckReport {
	/// A picture covering at least this share of a slide, in percent, carries
	/// content rather than decoration, so a recurrence of it is reported.
	const RECURRING_FLOOR: u32 = 25;

	/// Appends the triage to every deck read since the last run.
	pub(crate) fn append_all(world: &mut World) {
		let decks = world
			.query_filtered::<Entity, (With<Deck>, Without<DeckReported>)>()
			.iter(world)
			.collect::<Vec<_>>();
		for deck in decks {
			Self::append(world, deck);
			world.entity_mut(deck).insert(DeckReported);
		}
	}

	fn append(world: &mut World, root: Entity) {
		let deck = world
			.entity(root)
			.get::<Deck>()
			.cloned()
			.unwrap_or_default();
		let mut slides =
			world.with_state::<(
				Query<&Children>,
				Query<&Slide>,
				Query<&SlidePicture>,
				AttributeQuery,
			), _>(|(children, slides, pictures, attributes)| {
				children
					.iter_descendants_depth_first(root)
					.filter_map(|section| {
						let slide = slides.get(section).ok()?.clone();
						let pictures = children
							.iter_descendants_depth_first(section)
							.filter_map(|image| {
								let picture = pictures.get(image).ok()?.clone();
								let source = attributes
									.find(image, "src")
									.map(|(_, value)| {
										SmolStr::new(value.to_string())
									})
									.unwrap_or_default();
								Some((source, picture))
							})
							.collect();
						Some(SlideReport {
							section,
							slide,
							pictures,
							duplicate_of: None,
						})
					})
					.collect::<Vec<_>>()
			});
		let mut seen = HashMap::<String, u32>::default();
		for slide in &mut slides {
			let key = slide.slide.fingerprint.clone();
			if !key.is_empty() {
				slide.duplicate_of = seen.get(&key).copied();
				seen.entry(key).or_insert(slide.slide.index);
			}
		}
		let header = Self::header(world, &deck, slides.len());
		world.entity_mut(root).insert_child(0, header);
		for slide in &slides {
			Self::decorate(world, slide);
		}
		let report = Self::report(world, &deck, &slides);
		world.entity_mut(root).add_child(report);
	}

	fn header(world: &mut World, deck: &Deck, slides: usize) -> Entity {
		let line = format!(
			"{slides} slides, {:.2} x {:.2} inches.",
			deck.width as f64 / 914_400.,
			deck.height as f64 / 914_400.
		);
		world
			.spawn((Element::new("header"), children![(
				Element::new("p"),
				children![Value::Str(line.into())]
			)]))
			.id()
	}

	/// A slide's note where it has no words, after its heading, and its
	/// picture inventory before its notes.
	fn decorate(world: &mut World, slide: &SlideReport) {
		if slide.slide.words == 0 && slide.slide.tables == 0 {
			let none = world
				.spawn((Element::new("p"), children![(
					Element::new("em"),
					children![Value::str("no text")]
				)]))
				.id();
			world.entity_mut(slide.section).insert_child(2, none);
		}
		if slide.pictures.is_empty() {
			return;
		}
		let heading = world
			.spawn((Element::new("h3"), children![Value::str("Pictures")]))
			.id();
		let mut rows = vec![
			["#", "source", "native px", "covers"]
				.map(String::from)
				.to_vec(),
		];
		for (number, (source, picture)) in slide.pictures.iter().enumerate() {
			rows.push(vec![
				(number + 1).to_string(),
				source.to_string(),
				picture.size.to_string(),
				format!("{}%", picture.cover),
			]);
		}
		let table = Self::table(world, &rows);
		// the inventory sits before the notes, the section's last part
		let notes = world
			.with_state::<(Query<&Children>, Query<(), With<SourcePart>>), _>(
				|(children, parts)| {
					children.get(slide.section).ok().and_then(|children| {
						children.iter().position(|child| parts.contains(child))
					})
				},
			);
		let mut section = world.entity_mut(slide.section);
		match notes {
			Some(index) => section.insert_children(index, &[heading, table]),
			None => section.add_children(&[heading, table]),
		};
	}

	fn report(
		world: &mut World,
		deck: &Deck,
		slides: &[SlideReport],
	) -> Entity {
		let mut paragraphs = Vec::<String>::new();
		let hidden = slides
			.iter()
			.filter(|slide| slide.slide.hidden)
			.map(|slide| slide.slide.index.to_string())
			.collect::<Vec<_>>();
		if !hidden.is_empty() {
			paragraphs.push(format!(
				"hidden slide(s) {}: in the deck and in this dump, but not shown in a slideshow \
				 and dropped from a render. Transcribe from the dump and say it is hidden.",
				hidden.join(", ")
			));
		}
		if !deck.unlisted.is_empty() {
			paragraphs.push(format!(
				"{} slide part(s) the presentation never shows ({}): a deleted slide's leftovers, \
				 not a slide. Mention once, transcribe nothing.",
				deck.unlisted.len(),
				deck.unlisted
					.iter()
					.map(|path| path.rsplit('/').next().unwrap_or(path))
					.collect::<Vec<_>>()
					.join(", ")
			));
		}
		let blank_zero = |count: usize| match count {
			0 => String::new(),
			count => count.to_string(),
		};
		let mut rows = vec![
			[
				"slide",
				"part",
				"words",
				"pics",
				"largest",
				"tables",
				"notes",
				"carried by",
			]
			.map(String::from)
			.to_vec(),
		];
		for slide in slides {
			rows.push(vec![
				slide.slide.index.to_string(),
				slide
					.slide
					.part
					.rsplit('/')
					.next()
					.unwrap_or_default()
					.to_string(),
				slide.slide.words.to_string(),
				slide.pictures.len().to_string(),
				format!("{}%", Self::largest(slide)),
				blank_zero(slide.slide.tables),
				blank_zero(slide.slide.notes_words),
				match slide.duplicate_of {
					Some(first) => format!("duplicate of slide {first}"),
					None => Self::carried_by(slide).to_string(),
				},
			]);
		}
		let mut after = Vec::<String>::new();
		// a picture big enough to carry content, shown on several slides
		let mut recurring = BTreeMap::<&str, Vec<u32>>::new();
		for slide in slides {
			let mut sources = slide
				.pictures
				.iter()
				.filter(|(_, picture)| picture.cover >= Self::RECURRING_FLOOR)
				.map(|(source, _)| source.as_str())
				.collect::<Vec<_>>();
			sources.sort();
			sources.dedup();
			for source in sources {
				recurring.entry(source).or_default().push(slide.slide.index);
			}
		}
		for (source, on) in recurring.iter().filter(|(_, on)| on.len() >= 2) {
			let span = match on.windows(2).all(|pair| pair[1] == pair[0] + 1) {
				true => format!("{}-{}", on[0], on[on.len() - 1]),
				false => on
					.iter()
					.map(ToString::to_string)
					.collect::<Vec<_>>()
					.join(", "),
			};
			after.push(format!(
				"{source} is on {} slides ({span}): the same picture twice is one artefact \
				 re-shown, often with a different box drawn on it. Transcribe it once and \
				 cross-reference.",
				on.len()
			));
		}
		// one capture saved twice: two part names of identical dimensions
		let mut by_size = BTreeMap::<&str, BTreeMap<&str, Vec<u32>>>::new();
		for slide in slides {
			for (source, picture) in
				slide.pictures.iter().filter(|(_, picture)| {
					picture.cover >= Self::RECURRING_FLOOR
						&& picture.cover < 90
						&& !picture.size.is_empty()
				}) {
				by_size
					.entry(&picture.size)
					.or_default()
					.entry(source)
					.or_default()
					.push(slide.slide.index);
			}
		}
		for (size, names) in by_size
			.iter()
			.filter(|(_, names)| (2..=3).contains(&names.len()))
		{
			let at = names
				.iter()
				.map(|(name, on)| {
					format!(
						"{name} on {}",
						on.iter()
							.map(ToString::to_string)
							.collect::<Vec<_>>()
							.join(", ")
					)
				})
				.collect::<Vec<_>>()
				.join("; ");
			after.push(format!(
				"{} pictures of the same size, {size} ({at}): often one capture saved twice. \
				 Compare them before transcribing both.",
				names.len()
			));
		}
		let pictured = slides
			.iter()
			.filter(|slide| {
				slide.duplicate_of.is_none()
					&& Self::carried_by(slide) == "picture"
			})
			.map(|slide| slide.slide.index.to_string())
			.collect::<Vec<_>>();
		if !pictured.is_empty() {
			after.push(format!(
				"read and transcribe these slides' pictures: {}",
				pictured.join(", ")
			));
		}
		let duplicates = slides
			.iter()
			.filter_map(|slide| {
				slide
					.duplicate_of
					.map(|first| format!("{}=={first}", slide.slide.index))
			})
			.collect::<Vec<_>>();
		if !duplicates.is_empty() {
			after.push(format!(
				"duplicate slides (say so once, do not transcribe twice): {}",
				duplicates.join(", ")
			));
		}
		let section = world.spawn(Element::new("section")).id();
		let heading = world
			.spawn((Element::new("h2"), children![Value::str("Report")]))
			.id();
		let before = Self::paragraphs(world, &paragraphs);
		let table = Self::table(world, &rows);
		let after = Self::paragraphs(world, &after);
		let mut section_mut = world.entity_mut(section);
		section_mut.add_child(heading);
		section_mut.add_children(&before);
		section_mut.add_child(table);
		section_mut.add_children(&after);
		section
	}

	fn paragraphs(world: &mut World, texts: &[String]) -> Vec<Entity> {
		texts
			.iter()
			.map(|text| {
				world
					.spawn((Element::new("p"), children![Value::Str(
						text.as_str().into()
					)]))
					.id()
			})
			.collect()
	}

	/// A table of plain cells headed by its first row.
	fn table(world: &mut World, rows: &[Vec<String>]) -> Entity {
		let table = world.spawn(Element::new("table")).id();
		for (index, row) in rows.iter().enumerate() {
			let tag = match index {
				0 => "th",
				_ => "td",
			};
			let tr = world.spawn((Element::new("tr"), ChildOf(table))).id();
			for cell in row {
				world.spawn((Element::new(tag), ChildOf(tr), children![
					Value::Str(cell.as_str().into())
				]));
			}
		}
		table
	}

	fn largest(slide: &SlideReport) -> u32 {
		slide
			.pictures
			.iter()
			.map(|(_, picture)| picture.cover)
			.max()
			.unwrap_or(0)
	}

	/// Which half of the slide holds the content: a picture covering the
	/// whole slide under real text is a background photo, one covering half
	/// of it is a diagram or a screenshot to be read.
	fn carried_by(slide: &SlideReport) -> &'static str {
		let total = slide
			.pictures
			.iter()
			.map(|(_, picture)| picture.cover)
			.sum::<u32>();
		match (
			slide.slide.words,
			slide.pictures.len(),
			Self::largest(slide),
		) {
			(0, 0, _) => "blank",
			(words, _, _) if words < 25 => "picture",
			(_, _, largest) if largest >= 90 => "text over photo",
			(words, _, largest)
				if largest >= 40 || (total >= 25 && words < 40) =>
			{
				"picture"
			}
			_ => "text",
		}
	}
}
