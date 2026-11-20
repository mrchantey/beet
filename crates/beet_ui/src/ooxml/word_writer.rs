//! [`WordWriter`], a Word file's edits written back by the reverse of its
//! reading.
use super::node::OoxmlRead;
use super::ooxml_query::*;
use crate::prelude::*;
use beet_core::prelude::*;

type Ns = OoxmlNamespace;

/// The re-projection of a Word file's edits, what each element reverses to
/// listed on [`OoxmlRenderer`].
pub(super) struct WordWriter;


impl WordWriter {
	/// The empty and the ticked checkbox glyphs, when a control names none.
	const BOX: char = '\u{2610}';
	const TICKED: char = '\u{2612}';

	/// The elements an edit may add that a Word file is not written with,
	/// each written as its text.
	const DROPPED: [&str; 4] = ["table", "img", "ins", "del"];

	/// Writes every edit under `root` into nodes of the file, adding the
	/// relationships new links need to `file`. Each pass reads what those
	/// before it wrote: rows and cells first, so their words find a place,
	/// then the paragraphs those words are enclosed in, then runs.
	pub fn reproject(
		world: &mut World,
		root: Entity,
		file: &mut OoxmlFile,
	) -> Result {
		Self::warn_dropped(world, root);
		Self::tables(world, root);
		Self::enclose(world, root);
		let models = Self::paragraphs(world, root);
		Self::lines(world, root);
		let mut run_models = Self::lift(world, root);
		run_models.extend(Self::links(world, root, file)?);
		Self::texts(world, root, &models, &run_models);
		Self::breaks(world, root);
		Self::checkboxes(world, root);
		Self::cells(world, root);
		Ok(())
	}

	/// Whether `href` is an address outside the file, which a
	/// `w:hyperlink`'s relationship holds.
	fn is_external(href: &str) -> bool {
		!href.is_empty() && !href.starts_with('#')
	}

	/// Logs one warning naming what [`dropped`](Self::dropped) finds.
	fn warn_dropped(world: &mut World, root: Entity) {
		let dropped = Self::dropped(world, root);
		if !dropped.is_empty() {
			warn!(
				"a Word file is not written with the new `{}`: written as their text",
				dropped.join("`, `")
			);
		}
	}

	/// What an edit added whose structure is dropped, each named once: a new
	/// [`DROPPED`](Self::DROPPED) element, and a link to a place in the file.
	fn dropped(world: &mut World, root: Entity) -> Vec<&'static str> {
		world.with_state::<(OoxmlQuery, ElementQuery), _>(|(tree, elements)| {
			let mut dropped = Vec::<&'static str>::new();
			for entity in tree.descendants(root) {
				let Ok(view) = elements.get(entity) else {
					continue;
				};
				let name = match view.tag() {
					"a" if view.attribute_string("href").starts_with('#') => {
						"a href=\"#..\""
					}
					_ if tree.element(entity).is_some() => continue,
					tag => {
						match Self::DROPPED.iter().find(|name| **name == tag) {
							Some(name) => *name,
							None => continue,
						}
					}
				};
				if tree.within_source(entity) && !dropped.contains(&name) {
					dropped.push(name);
				}
			}
			dropped
		})
	}

	/// Writes every row and cell an edit added to a table the file holds as
	/// a `w:tr` and a `w:tc`: a row with the properties of the nearest row
	/// the file holds, never a repeated header, a cell with those of the
	/// cell in its column in the nearest other row, else of the cell before
	/// it, never merged down and spanning its `colspan`. A table a row
	/// outgrows has its grid widened.
	fn tables(world: &mut World, root: Entity) {
		let (rows, cells) = world.with_state::<(OoxmlQuery, ElementQuery), _>(
			|(tree, elements)| {
				let order = DocumentOrder::new(&tree, root);
				let tag_of = |entity: Entity| {
					elements
						.get(entity)
						.ok()
						.map(|view| SmolStr::new(view.tag()))
				};
				let is_new = |entity: Entity, tag: &str| {
					tree.element(entity).is_none()
						&& tag_of(entity).is_some_and(|own| own == tag)
				};
				// the table the file holds that `entity` sits in, when its
				// nearest table is one
				let held_table = |entity: Entity| {
					tree.ancestors(entity)
						.into_iter()
						.find(|ancestor| {
							tag_of(*ancestor).as_deref() == Some("table")
						})
						.filter(|table| tree.is(*table, Ns::WORD, "tbl"))
				};
				let held_rows = |table: Entity| {
					tree.descendants(table)
						.into_iter()
						.filter(|row| tree.is(*row, Ns::WORD, "tr"))
						.filter(|row| {
							tree.ancestor(*row, Ns::WORD, "tbl") == Some(table)
						})
						.collect::<Vec<_>>()
				};
				let rows = tree
					.descendants(root)
					.into_iter()
					.filter(|row| is_new(*row, "tr"))
					.filter_map(|row| {
						let table = held_table(row)?;
						let model = order.nearest(row, held_rows(table));
						Some(NewRow {
							row,
							named: tree.element(table)?.clone(),
							properties: model.and_then(|model| {
								tree.child(model, Ns::WORD, "trPr")
							}),
						})
					})
					.collect::<Vec<_>>();
				let cells = tree
					.descendants(root)
					.into_iter()
					.filter(|cell| {
						tree.element(*cell).is_none()
							&& tag_of(*cell)
								.is_some_and(|tag| TableGrid::is_cell_tag(&tag))
					})
					.filter_map(|cell| {
						let row = tree.ancestors(cell).into_iter().find(
							|ancestor| {
								tag_of(*ancestor).as_deref() == Some("tr")
							},
						)?;
						let table = held_table(row)?;
						let row_is_written = tree.is(row, Ns::WORD, "tr")
							|| rows.iter().any(|new| new.row == row);
						if !row_is_written {
							return None;
						}
						let column =
							Self::column_of(&tree, &elements, row, cell);
						let others = held_rows(table)
							.into_iter()
							.filter(|other| *other != row)
							.collect::<Vec<_>>();
						let model = order
							.nearest(row, others)
							.and_then(|other| {
								Self::cell_at(&tree, &elements, other, column)
							})
							.or_else(|| {
								// the cell before it in its own row
								tree.children(row)
									.into_iter()
									.take_while(|sibling| *sibling != cell)
									.filter(|sibling| {
										tree.is(*sibling, Ns::WORD, "tc")
									})
									.last()
							});
						Some(NewCell {
							cell,
							named: tree.element(table)?.clone(),
							properties: model.and_then(|model| {
								tree.child(model, Ns::WORD, "tcPr")
							}),
							span: elements.get(cell).ok()?.colspan(),
							table,
						})
					})
					.collect::<Vec<_>>();
				(rows, cells)
			},
		);
		for NewRow {
			row,
			named,
			properties,
		} in rows
		{
			world
				.entity_mut(row)
				.insert(OoxmlNode::Element(named.sibling(Ns::WORD, "tr")));
			if let Some(properties) = properties {
				let copy = clone_nodes(world, properties, row);
				world.entity_mut(row).insert_child(0, copy);
				Self::remove_properties(world, copy, &["tblHeader"]);
			}
		}
		let mut tables = Vec::new();
		for NewCell {
			cell,
			named,
			properties,
			span,
			table,
		} in cells
		{
			world
				.entity_mut(cell)
				.insert(OoxmlNode::Element(named.sibling(Ns::WORD, "tc")));
			if let Some(properties) = properties {
				let copy = clone_nodes(world, properties, cell);
				world.entity_mut(cell).insert_child(0, copy);
				Self::remove_properties(world, copy, &[
					"gridSpan", "hMerge", "vMerge",
				]);
			}
			if span > 1 {
				Self::add_properties(world, cell, &PropertyList::CELL, &[(
					"gridSpan",
					Some(span.to_string().into()),
				)]);
			}
			if !tables.contains(&table) {
				tables.push(table);
			}
		}
		for table in tables {
			Self::widen_grid(world, table);
		}
	}

	/// The visual column `cell` starts at in `row`, from 0: the spans of
	/// the cells before it.
	fn column_of(
		tree: &OoxmlQuery,
		elements: &ElementQuery,
		row: Entity,
		cell: Entity,
	) -> u32 {
		tree.children(row)
			.into_iter()
			.take_while(|sibling| *sibling != cell)
			.filter_map(|sibling| elements.get(sibling).ok())
			.filter(|view| TableGrid::is_cell_tag(view.tag()))
			.map(|view| view.colspan())
			.sum()
	}

	/// The cell of `row` covering the visual `column`, from 0.
	fn cell_at(
		tree: &OoxmlQuery,
		elements: &ElementQuery,
		row: Entity,
		column: u32,
	) -> Option<Entity> {
		let mut start = 0;
		for cell in tree.children(row) {
			let Some(view) = elements
				.get(cell)
				.ok()
				.filter(|view| TableGrid::is_cell_tag(view.tag()))
			else {
				continue;
			};
			let span = view.colspan();
			if (start..start + span).contains(&column) {
				return Some(cell);
			}
			start += span;
		}
		None
	}

	/// Widens the grid of `table`, a `w:tbl`, to its widest row, each column
	/// it gains a copy of its last.
	fn widen_grid(world: &mut World, table: Entity) {
		let (grid, columns, widest) = world
			.with_state::<(OoxmlQuery, ElementQuery), _>(|(tree, elements)| {
				let grid = tree.child(table, Ns::WORD, "tblGrid");
				let columns = grid
					.map(|grid| tree.children_named(grid, Ns::WORD, "gridCol"))
					.unwrap_or_default();
				let widest = tree
					.children_named(table, Ns::WORD, "tr")
					.into_iter()
					.map(|row| {
						tree.children(row)
							.into_iter()
							.filter_map(|cell| elements.get(cell).ok())
							.filter(|view| TableGrid::is_cell_tag(view.tag()))
							.map(|view| view.colspan())
							.sum::<u32>()
					})
					.max()
					.unwrap_or_default();
				(grid, columns, widest)
			});
		let (Some(grid), Some(last)) = (grid, columns.last().copied()) else {
			return;
		};
		for _ in columns.len() as u32..widest {
			clone_nodes(world, last, grid);
		}
	}

	/// Wraps each run of loose content, new words or a new phrasing element
	/// where a paragraph belongs, ie straight under the body or in a cell, in
	/// a new `p`, since Word holds every word in a paragraph. Whitespace at
	/// a run's edges is left out, so whitespace alone makes no paragraph.
	fn enclose(world: &mut World, root: Entity) {
		let groups = world
			.with_state::<(OoxmlQuery, Query<&Element>, Query<&Value, NewText>), _>(
				|(tree, elements, texts)| {
					let tag_of = |entity: Entity| {
						elements.get(entity).ok().map(Element::tag)
					};
					let is_loose = |entity: Entity| {
						let is_new = texts.contains(entity)
							|| (tree.element(entity).is_none()
								&& tag_of(entity).is_some_and(|tag| {
									Element::INLINE_ELEMENTS.contains(&tag)
								}));
						let parent_holds_paragraphs =
							tree.parent(entity).is_some_and(|parent| {
								!tag_of(parent).is_some_and(|tag| {
									matches!(tag, "table" | "tr")
								})
							});
						// nothing above it is a paragraph, phrasing, or never written
						let placed = tree.ancestors(entity).into_iter().all(
							|ancestor| {
								!tree.is(ancestor, Ns::WORD, "p")
									&& !tag_of(ancestor).is_some_and(|tag| {
										DocumentText::is_block(tag)
											|| Element::INLINE_ELEMENTS
												.contains(&tag) || tag == "caption"
											|| RenderTreeQuery::is_textless(tag)
									})
							},
						);
						is_new
							&& parent_holds_paragraphs
							&& placed && tree.within_source(entity)
					};
					let is_blank = |entity: Entity| {
						texts.get(entity).is_ok_and(|value| {
							value.to_string().trim().is_empty()
						})
					};
					let mut groups = Vec::<(Entity, Vec<Entity>)>::new();
					for parent in
						core::iter::once(root).chain(tree.descendants(root))
					{
						// whitespace at a paragraph's edges is no reader's
						let mut flush = |group: &mut Vec<Entity>| {
							let start = group
								.iter()
								.position(|entity| !is_blank(*entity));
							let end = group
								.iter()
								.rposition(|entity| !is_blank(*entity));
							if let (Some(start), Some(end)) = (start, end) {
								groups.push((
									parent,
									group[start..=end].to_vec(),
								));
							}
							group.clear();
						};
						let mut group = Vec::new();
						for child in tree.children(parent) {
							match is_loose(child) {
								true => group.push(child),
								false => flush(&mut group),
							}
						}
						flush(&mut group);
					}
					groups
				},
			);
		for (parent, group) in groups {
			let index = world
				.entity(parent)
				.get::<Children>()
				.and_then(|children| {
					children.iter().position(|child| child == group[0])
				})
				.unwrap_or_default();
			let paragraph = world.spawn(Element::new("p")).id();
			world.entity_mut(parent).insert_child(index, paragraph);
			world.entity_mut(paragraph).add_children(&group);
		}
	}

	/// Splits every new text inside a `pre` at its line ends, a new `br`
	/// between each line, so its lines stay lines.
	fn lines(world: &mut World, root: Entity) {
		let found = world
			.with_state::<(OoxmlQuery, Query<&Element>, Query<&Value, NewText>), _>(
				|(tree, elements, texts)| {
					tree.descendants(root)
						.into_iter()
						.filter_map(|text| {
							let value = texts.get(text).ok()?.as_str().ok()?;
							let lines = value
								.strip_suffix('\n')
								.unwrap_or(value)
								.split('\n')
								.map(SmolStr::new)
								.collect::<Vec<_>>();
							let in_pre = tree.ancestors(text).into_iter().any(
								|ancestor| {
									elements.get(ancestor).is_ok_and(
										|element| {
											Element::PRE_ELEMENTS
												.contains(&element.tag())
										},
									)
								},
							);
							(in_pre && lines.len() > 1)
								.then(|| {
									Some((text, tree.parent(text)?, lines))
								})
								.flatten()
						})
						.collect::<Vec<_>>()
				},
			);
		for (text, parent, lines) in found {
			let mut lines = lines.into_iter();
			let first = lines.next().unwrap_or_default();
			world.entity_mut(text).insert(Value::Str(first));
			let mut index = world
				.entity(parent)
				.get::<Children>()
				.and_then(|children| {
					children.iter().position(|child| child == text)
				})
				.unwrap_or_default();
			for line in lines {
				let brk = world.spawn(Element::new("br")).id();
				let words = world.spawn(Value::Str(line)).id();
				world
					.entity_mut(parent)
					.insert_children(index + 1, &[brk, words]);
				index += 2;
			}
		}
	}

	/// Lifts every new word or break an edit put inside a run the file holds
	/// out of it, since a run holds no run: the run keeps what came before,
	/// the new content follows in its own run, the elements between it and
	/// the run copied around it, and what came after moves to a copy of the
	/// run after that. Answers each lifted content's run properties, the
	/// run's own, by the top of what was lifted.
	fn lift(
		world: &mut World,
		root: Entity,
	) -> HashMap<Entity, Option<Entity>> {
		let leaves = world.with_state::<(
			OoxmlQuery,
			Query<&Element, Without<OoxmlNode>>,
			Query<(), NewText>,
		), _>(|(tree, new_elements, texts)| {
			tree.descendants(root)
				.into_iter()
				.filter(|entity| {
					texts.contains(*entity)
						|| new_elements
							.get(*entity)
							.is_ok_and(|element| element.tag() == "br")
				})
				.filter(|entity| !Self::in_template(&tree, *entity))
				.filter(|entity| {
					tree.ancestor(*entity, Ns::WORD, "r").is_some()
				})
				.collect::<Vec<_>>()
		});
		let mut lifted = HashMap::default();
		for leaf in leaves {
			// earlier lifts move nodes, so each reads the tree afresh
			let Some((run, path, place, properties)) = world
				.with_state::<OoxmlQuery, _>(|tree| {
					let run = tree.ancestor(leaf, Ns::WORD, "r")?;
					let mut path = tree
						.ancestors(leaf)
						.into_iter()
						.take_while(|ancestor| *ancestor != run)
						.collect::<Vec<_>>();
					path.reverse();
					Some((
						run,
						path,
						tree.parent(run)?,
						tree.child(run, Ns::WORD, "rPr"),
					))
				})
			else {
				continue;
			};
			// what follows the leaf at each level, gathered into copies from
			// the innermost out
			let nodes = core::iter::once(run)
				.chain(path.iter().copied())
				.collect::<Vec<_>>();
			let mut tail = None;
			for (depth, node) in nodes.iter().enumerate().rev() {
				let child = nodes.get(depth + 1).copied().unwrap_or(leaf);
				let after = world
					.entity(*node)
					.get::<Children>()
					.map(|children| {
						children
							.iter()
							.skip_while(|sibling| *sibling != child)
							.skip(1)
							.collect::<Vec<_>>()
					})
					.unwrap_or_default();
				if tail.is_none() && after.is_empty() {
					continue;
				}
				let copy = Self::copy_shell(world, *node);
				if let Some(tail) = tail {
					world.entity_mut(copy).add_child(tail);
				}
				world.entity_mut(copy).add_children(&after);
				tail = Some(copy);
			}
			// the leaf in copies of the elements between it and the run
			let mut top = leaf;
			for node in path.iter().rev() {
				let shell = Self::copy_shell(world, *node);
				world.entity_mut(shell).add_child(top);
				top = shell;
			}
			let index = world
				.entity(place)
				.get::<Children>()
				.and_then(|children| {
					children.iter().position(|child| child == run)
				})
				.unwrap_or_default();
			let placed = core::iter::once(top).chain(tail).collect::<Vec<_>>();
			world.entity_mut(place).insert_children(index + 1, &placed);
			// an element left holding nothing goes
			for node in path.iter().rev() {
				let empty = world
					.entity(*node)
					.get::<Children>()
					.is_none_or(|children| children.is_empty());
				if empty && !world.entity(*node).contains::<OoxmlNode>() {
					world.entity_mut(*node).despawn();
				}
			}
			lifted.insert(top, properties);
		}
		lifted
	}

	/// A copy of `node` without its children: its element and attributes,
	/// its node, and a run's properties.
	fn copy_shell(world: &mut World, node: Entity) -> Entity {
		let entity = world.entity(node);
		let element = entity.get::<Element>().cloned();
		let source = entity.get::<OoxmlNode>().cloned();
		let attributes = entity
			.get::<Attributes>()
			.map(|attributes| {
				attributes
					.iter()
					.filter_map(|attribute| {
						let attribute = world.entity(attribute);
						Some((
							attribute.get::<Attribute>()?.clone(),
							attribute.get::<Value>()?.clone(),
						))
					})
					.collect::<Vec<_>>()
			})
			.unwrap_or_default();
		let properties = world.with_state::<OoxmlQuery, _>(|tree| {
			tree.is(node, Ns::WORD, "r")
				.then(|| tree.child(node, Ns::WORD, "rPr"))
				.flatten()
		});
		let copy = world.spawn_empty().id();
		if let Some(element) = element {
			world.entity_mut(copy).insert(element);
		}
		if let Some(source) = source {
			world.entity_mut(copy).insert(source);
		}
		for (attribute, value) in attributes {
			world.spawn((AttributeOf::new(copy), attribute, value));
		}
		if let Some(properties) = properties {
			clone_nodes(world, properties, copy);
		}
		copy
	}

	/// Writes every new `br` in a paragraph as a run holding a `w:br`.
	fn breaks(world: &mut World, root: Entity) {
		let found = world
			.with_state::<(OoxmlQuery, Query<&Element, Without<OoxmlNode>>), _>(
				|(tree, new_elements)| {
					tree.descendants(root)
						.into_iter()
						.filter(|entity| {
							new_elements
								.get(*entity)
								.is_ok_and(|element| element.tag() == "br")
						})
						.filter_map(|brk| {
							let paragraph =
								tree.ancestor(brk, Ns::WORD, "p")?;
							Some((brk, tree.element(paragraph)?.clone()))
						})
						.collect::<Vec<_>>()
				},
			);
		for (brk, named) in found {
			world
				.entity_mut(brk)
				.insert(OoxmlNode::Element(named.sibling(Ns::WORD, "r")));
			world.spawn((
				OoxmlNode::Element(named.sibling(Ns::WORD, "br")),
				ChildOf(brk),
			));
		}
	}

	/// Whether `entity`'s parent is a text element, a `w:t`, `w:delText` or
	/// `w:instrText`, so its words are that element's.
	fn in_template(tree: &OoxmlQuery, entity: Entity) -> bool {
		tree.parent(entity)
			.and_then(|parent| tree.element(parent))
			.is_some_and(|parent| {
				matches!(parent.local_name(), "t" | "delText" | "instrText")
			})
	}

	/// Removes the children of the properties element `container` named
	/// any of `locals`, and the container when that leaves it empty.
	fn remove_properties(
		world: &mut World,
		container: Entity,
		locals: &[&str],
	) {
		let (removed, emptied) = world.with_state::<OoxmlQuery, _>(|tree| {
			let children = tree.children(container);
			let removed = children
				.iter()
				.copied()
				.filter(|child| {
					tree.element(*child).is_some_and(|element| {
						locals.contains(&element.local_name())
					})
				})
				.collect::<Vec<_>>();
			let emptied = removed.len() == children.len();
			(removed, emptied)
		});
		for property in removed {
			world.entity_mut(property).despawn();
		}
		if emptied {
			world.entity_mut(container).despawn();
		}
	}

	/// Makes every new paragraph a `w:p` with the properties of the nearest
	/// file-held paragraph of its own tag, a sibling first, else the nearest
	/// in its part. With none, a heading takes its `Heading<n>` style and
	/// any other its first sibling paragraph's properties. Answers each new
	/// paragraph's model, whose runs its new words take their look from.
	fn paragraphs(world: &mut World, root: Entity) -> HashMap<Entity, Entity> {
		let found = world.with_state::<(OoxmlQuery, Query<&Element>), _>(
			|(tree, elements)| {
				let order = DocumentOrder::new(&tree, root);
				let tag_of = |entity: Entity| {
					elements.get(entity).ok().map(Element::tag)
				};
				let held = tree
					.descendants(root)
					.into_iter()
					.filter(|entity| tree.is(*entity, Ns::WORD, "p"))
					.collect::<Vec<_>>();
				tree.descendants(root)
					.into_iter()
					.filter_map(|entity| {
						let tag = tag_of(entity)?;
						let is_new = tree.element(entity).is_none()
							&& tree.within_source(entity)
							&& DocumentText::is_block(tag);
						if !is_new {
							return None;
						}
						let siblings = tree
							.parent(entity)
							.map(|parent| tree.children(parent))
							.unwrap_or_default();
						let same_tag = |candidate: &Entity| {
							tree.is(*candidate, Ns::WORD, "p")
								&& tag_of(*candidate) == Some(tag)
						};
						let part = tree.part_of(entity);
						let model = order
							.nearest(
								entity,
								siblings.iter().copied().filter(same_tag),
							)
							.or_else(|| {
								order.nearest(
									entity,
									held.iter()
										.copied()
										.filter(same_tag)
										.filter(|candidate| {
											tree.part_of(*candidate) == part
										}),
								)
							});
						let first_sibling = siblings
							.iter()
							.copied()
							.find(|sibling| tree.is(*sibling, Ns::WORD, "p"));
						let properties = |paragraph: Entity| {
							tree.child(paragraph, Ns::WORD, "pPr")
						};
						let written =
							match (model, DocumentText::heading_level(tag)) {
								(Some(model), _) => NewParagraph::Like {
									model,
									properties: properties(model),
								},
								(None, Some(level)) => {
									NewParagraph::Heading(level)
								}
								(None, None) => NewParagraph::Fallback {
									model: first_sibling,
									properties: first_sibling
										.and_then(properties),
									is_item: tag == "li",
								},
							};
						let named =
							tree.ancestors(entity).into_iter().find_map(
								|ancestor| tree.element(ancestor).cloned(),
							);
						Some((entity, written, named))
					})
					.collect::<Vec<_>>()
			},
		);
		let mut models = HashMap::default();
		for (entity, written, named) in found {
			let named = named.unwrap_or_else(|| {
				OoxmlElement::new("w:p", Some(Ns::WORD.into()))
			});
			let paragraph = named.sibling(Ns::WORD, "p");
			world
				.entity_mut(entity)
				.insert(OoxmlNode::Element(paragraph.clone()));
			let (model, properties) = match written {
				NewParagraph::Like { model, properties } => {
					(Some(model), properties)
				}
				NewParagraph::Heading(level) => {
					let mut style = paragraph.sibling(Ns::WORD, "pStyle");
					style.set_attribute(style.own_attribute(
						"val",
						ParagraphStyle::heading(level).0,
					));
					let properties = world
						.spawn((
							OoxmlNode::Element(
								paragraph.sibling(Ns::WORD, "pPr"),
							),
							children![OoxmlNode::Element(style)],
						))
						.id();
					world.entity_mut(entity).insert_child(0, properties);
					(None, None)
				}
				NewParagraph::Fallback {
					model,
					properties,
					is_item,
				} => {
					if is_item {
						warn!(
							"a new list item has no list item to join: written as a paragraph, since no numbering is invented"
						);
					}
					(model, properties)
				}
			};
			if let Some(properties) = properties {
				let copy = clone_nodes(world, properties, entity);
				world.entity_mut(entity).insert_child(0, copy);
			}
			models.insert(entity, model.unwrap_or(entity));
		}
		models
	}

	/// Writes every new link to an address outside the file as a
	/// `w:hyperlink` of an external relationship its part holds, shared by
	/// every link to that address, points an existing link whose `href`
	/// changed at its new address, and writes a link to a place in the file
	/// as its text. Answers each new link's model run properties, those of
	/// the nearest link's run the file holds.
	fn links(
		world: &mut World,
		root: Entity,
		file: &mut OoxmlFile,
	) -> Result<HashMap<Entity, Option<Entity>>> {
		let found = world.with_state::<(OoxmlQuery, ElementQuery), _>(
			|(tree, elements)| {
				let order = DocumentOrder::new(&tree, root);
				let held = tree
					.descendants(root)
					.into_iter()
					.filter(|entity| tree.is(*entity, Ns::WORD, "hyperlink"))
					.collect::<Vec<_>>();
				tree.descendants(root)
					.into_iter()
					.filter(|entity| tree.within_source(*entity))
					.filter_map(|link| {
						let view = elements.get(link).ok()?;
						(view.tag() == "a").then_some(())?;
						let href = SmolStr::new(view.attribute_string("href"));
						let part = tree.part_of(link)?;
						let path = tree.path(part)?.clone();
						if tree.is(link, Ns::WORD, "hyperlink") {
							let id = tree
								.attribute(link, Some(Ns::RELATIONSHIPS), "id")
								.map(SmolStr::new);
							return Some(FoundLink::Held {
								link,
								href,
								path,
								id,
							});
						}
						let paragraph = tree.ancestor(link, Ns::WORD, "p")?;
						let model = order
							.nearest(
								link,
								held.iter().copied().filter(|candidate| {
									tree.part_of(*candidate) == Some(part)
								}),
							)
							.or_else(|| held.first().copied())
							.and_then(|model| {
								tree.descendant(model, Ns::WORD, "r")
							})
							.and_then(|run| tree.child(run, Ns::WORD, "rPr"));
						Some(FoundLink::New {
							link,
							href,
							path,
							named: tree.element(paragraph)?.clone(),
							prefix: tree
								.prefix_for(paragraph, Ns::RELATIONSHIPS)
								.map(SmolStr::new),
							model,
						})
					})
					.collect::<Vec<_>>()
			},
		);
		let mut link_runs = HashMap::default();
		for found in found {
			match found {
				FoundLink::Held { link, href, .. }
					if !Self::is_external(&href) =>
				{
					// its runs stay, written in place
					world.entity_mut(link).remove::<OoxmlNode>();
				}
				FoundLink::Held {
					link,
					href,
					path,
					id,
				} => {
					let target = file.part_at(&path).and_then(|part| {
						id.as_ref()
							.and_then(|id| file.hyperlinks(&part).remove(id))
					});
					if target.as_ref() == Some(&href) {
						continue;
					}
					let id = file.hyperlink_to(&path, &href)?;
					if let Some(mut node) =
						world.entity_mut(link).get_mut::<OoxmlNode>()
						&& let Some(element) = node.element_mut()
					{
						// the name is kept, the value replaced
						element.set_attribute(OoxmlAttribute::new(
							"r:id",
							Some(Ns::RELATIONSHIPS.into()),
							id,
						));
					}
				}
				FoundLink::New { href, .. } if !Self::is_external(&href) => {}
				FoundLink::New {
					link,
					href,
					path,
					named,
					prefix,
					model,
				} => {
					let id = file.hyperlink_to(&path, &href)?;
					let mut hyperlink = named.sibling(Ns::WORD, "hyperlink");
					let prefix = match prefix {
						Some(prefix) => prefix,
						None => {
							hyperlink.attributes.push(OoxmlAttribute::new(
								"xmlns:r",
								None,
								Ns::RELATIONSHIPS,
							));
							"r".into()
						}
					};
					hyperlink.attributes.push(OoxmlAttribute::new(
						format!("{prefix}:id"),
						Some(Ns::RELATIONSHIPS.into()),
						id,
					));
					world
						.entity_mut(link)
						.insert(OoxmlNode::Element(hyperlink));
					if model.is_some() {
						link_runs.insert(link, model);
					}
				}
			}
		}
		Ok(link_runs)
	}

	/// Puts every new text node in a run of its paragraph's first look, or
	/// of the run properties `run_models` gives it or an element around it,
	/// ie a new link's, adding what the new elements around it say, and
	/// keeps the spaces of every edited text.
	fn texts(
		world: &mut World,
		root: Entity,
		models: &HashMap<Entity, Entity>,
		run_models: &HashMap<Entity, Option<Entity>>,
	) {
		let preserve =
			OoxmlAttribute::new("xml:space", Some(Ns::XML.into()), "preserve");
		let texts = world
			.with_state::<(OoxmlQuery, Query<&Element>, Query<(), NewText>), _>(
				|(tree, elements, texts)| {
					tree.descendants(root)
						.into_iter()
						.filter(|entity| texts.contains(*entity))
						.filter_map(|text| {
							let parent = tree.parent(text)?;
							let in_t = Self::in_template(&tree, text);
							let paragraph =
								tree.ancestor(text, Ns::WORD, "p")?;
							let within = tree
								.ancestors(text)
								.into_iter()
								.take_while(|ancestor| *ancestor != paragraph)
								.collect::<Vec<_>>();
							let run_model = core::iter::once(&text)
								.chain(within.iter())
								.find_map(|entity| run_models.get(entity))
								.copied();
							let mut looks = Vec::<Property>::new();
							for tag in within
								.iter()
								.filter(|ancestor| {
									tree.element(**ancestor).is_none()
								})
								.filter_map(|ancestor| {
									elements.get(*ancestor).ok()
								})
								.map(Element::tag)
							{
								let look: Property = match tag {
									tag if TextFormatting::is_bold(tag) => {
										("b", None)
									}
									tag if TextFormatting::is_italic(tag) => {
										("i", None)
									}
									"mark" => {
										("highlight", Some("yellow".into()))
									}
									_ => continue,
								};
								if !looks.contains(&look) {
									looks.push(look);
								}
							}
							Some((
								text, parent, in_t, paragraph, run_model, looks,
							))
						})
						.collect::<Vec<_>>()
				},
			);
		for (text, parent, in_t, paragraph, run_model, looks) in texts {
			if in_t {
				if OoxmlRead::edited(world, text)
					&& let Some(mut node) =
						world.entity_mut(parent).get_mut::<OoxmlNode>()
					&& let Some(element) = node.element_mut()
				{
					element.set_attribute(preserve.clone());
				}
				continue;
			}
			let model = models.get(&paragraph).copied().unwrap_or(paragraph);
			let (run_properties, named) =
				world.with_state::<OoxmlQuery, _>(|tree| {
					let runs = tree.descendants_named(model, Ns::WORD, "r");
					let run = runs
						.iter()
						.find(|run| {
							tree.descendant(**run, Ns::WORD, "t").is_some()
						})
						.or(runs.first())
						.copied();
					(
						run_model.unwrap_or_else(|| {
							run.and_then(|run| tree.child(run, Ns::WORD, "rPr"))
						}),
						tree.element(paragraph).cloned(),
					)
				});
			let named = named.unwrap_or_else(|| {
				OoxmlElement::new("w:p", Some(Ns::WORD.into()))
			});
			let index = world
				.entity(parent)
				.get::<Children>()
				.and_then(|children| {
					children.iter().position(|child| child == text)
				})
				.unwrap_or_default();
			let run = world
				.spawn(OoxmlNode::Element(named.sibling(Ns::WORD, "r")))
				.id();
			if let Some(properties) = run_properties {
				clone_nodes(world, properties, run);
			}
			Self::add_properties(world, run, &PropertyList::RUN, &looks);
			let mut written = named.sibling(Ns::WORD, "t");
			written.set_attribute(preserve.clone());
			let element = world
				.spawn((OoxmlNode::Element(written), ChildOf(run)))
				.id();
			world.entity_mut(element).add_child(text);
			world.entity_mut(parent).insert_child(index, run);
		}
	}

	/// Writes each checkbox control's state as its `checked` says: the
	/// control's own value, its glyph, and a newly checked one's glyph bold
	/// and highlighted.
	fn checkboxes(world: &mut World, root: Entity) {
		let boxes = world
			.with_state::<(OoxmlQuery, DocumentText, Query<&Children>), _>(
				|(tree, text, children)| {
					children
						.iter_descendants_depth_first(root)
						.filter(|entity| tree.is(*entity, Ns::WORD, "sdt"))
						.filter_map(|control| {
							let checked = text.checkbox(control)?;
							let properties =
								tree.child(control, Ns::WORD, "sdtPr")?;
							let checkbox = tree.descendant(
								properties,
								Ns::WORD_2010,
								"checkbox",
							)?;
							let state =
								tree.child(checkbox, Ns::WORD_2010, "checked");
							let was = state
								.and_then(|state| {
									tree.attribute(
										state,
										Some(Ns::WORD_2010),
										"val",
									)
								})
								.is_some_and(|value| {
									matches!(value, "1" | "true")
								});
							let glyph = |local: &str, default: char| {
								tree.child(checkbox, Ns::WORD_2010, local)
									.and_then(|glyph| {
										tree.attribute(
											glyph,
											Some(Ns::WORD_2010),
											"val",
										)
									})
									.and_then(|code| {
										u32::from_str_radix(code, 16).ok()
									})
									.and_then(char::from_u32)
									.unwrap_or(default)
							};
							let content =
								tree.child(control, Ns::WORD, "sdtContent");
							let glyphs = content
								.map(|content| {
									tree.descendants_named(
										content,
										Ns::WORD,
										"t",
									)
									.into_iter()
									.flat_map(|written| tree.children(written))
									.collect::<Vec<_>>()
								})
								.unwrap_or_default();
							let runs = content
								.map(|content| {
									tree.descendants_named(
										content,
										Ns::WORD,
										"r",
									)
								})
								.unwrap_or_default();
							(checked != was).then(|| {
								(
									checked,
									checkbox,
									state,
									glyph("checkedState", Self::TICKED),
									glyph("uncheckedState", Self::BOX),
									glyphs,
									runs,
									tree.element(checkbox).cloned(),
								)
							})
						})
						.collect::<Vec<_>>()
				},
			);
		for (checked, checkbox, state, ticked, empty, glyphs, runs, named) in
			boxes
		{
			let Some(named) = named else { continue };
			let value =
				named.own_attribute("val", if checked { "1" } else { "0" });
			match state {
				Some(state) => {
					if let Some(mut node) =
						world.entity_mut(state).get_mut::<OoxmlNode>()
						&& let Some(element) = node.element_mut()
					{
						element.set_attribute(value);
					}
				}
				// the schema puts `checked` first
				None => {
					let mut element = named.sibling(Ns::WORD_2010, "checked");
					element.set_attribute(value);
					let state = world.spawn(OoxmlNode::Element(element)).id();
					world.entity_mut(checkbox).insert_child(0, state);
				}
			}
			let (from, to) = match checked {
				true => (empty, ticked),
				false => (ticked, empty),
			};
			for glyph in glyphs {
				if let Some(mut node) =
					world.entity_mut(glyph).get_mut::<OoxmlNode>()
					&& let OoxmlNode::Text(text) = &mut *node
				{
					*text = text.replacen(from, &to.to_string(), 1).into();
				}
			}
			if checked {
				for run in runs {
					Self::add_properties(world, run, &PropertyList::RUN, &[
						("b", None),
						("highlight", Some("yellow".into())),
					]);
				}
			}
		}
	}

	/// Adds each of `properties` to `owner`'s properties element `list`,
	/// made when it has none, in the order the schema fixes: a property
	/// already present is kept, a toggle switched on.
	fn add_properties(
		world: &mut World,
		owner: Entity,
		list: &PropertyList,
		properties: &[Property],
	) {
		if properties.is_empty() {
			return;
		}
		let (existing, named) = world.with_state::<OoxmlQuery, _>(|tree| {
			(
				tree.child(owner, Ns::WORD, list.local),
				tree.element(owner).cloned(),
			)
		});
		let Some(named) = named else { return };
		let container = existing.unwrap_or_else(|| {
			let container = world
				.spawn(OoxmlNode::Element(named.sibling(Ns::WORD, list.local)))
				.id();
			world.entity_mut(owner).insert_child(0, container);
			container
		});
		let rank = |local: &str| {
			list.order
				.iter()
				.position(|name| *name == local)
				.unwrap_or(list.order.len())
		};
		for (local, value) in properties {
			let present = world.with_state::<OoxmlQuery, _>(|tree| {
				tree.children(container)
					.into_iter()
					.map(|child| {
						let local = tree
							.element(child)
							.map(|element| element.local_name().to_owned())
							.unwrap_or_default();
						(child, local)
					})
					.collect::<Vec<_>>()
			});
			if let Some((property, _)) =
				present.iter().find(|(_, existing)| existing == local)
			{
				// a toggle written off, ie `w:b w:val="0"`, is switched on
				if value.is_none()
					&& let Some(mut node) =
						world.entity_mut(*property).get_mut::<OoxmlNode>()
					&& let Some(element) = node.element_mut()
				{
					element.remove_attribute(Some(Ns::WORD), "val");
				}
				continue;
			}
			let mut property = named.sibling(Ns::WORD, local);
			if let Some(value) = value {
				property.set_attribute(
					property.own_attribute("val", value.clone()),
				);
			}
			let index = present
				.iter()
				.position(|(_, existing)| rank(existing) > rank(local))
				.unwrap_or(present.len());
			let property = world.spawn(OoxmlNode::Element(property)).id();
			world.entity_mut(container).insert_child(index, property);
		}
	}

	/// Keeps a paragraph in every cell, since Word holds a cell to one.
	fn cells(world: &mut World, root: Entity) {
		let empty = world.with_state::<(OoxmlQuery, Query<&Children>), _>(
			|(tree, children)| {
				children
					.iter_descendants_depth_first(root)
					.filter(|entity| tree.is(*entity, Ns::WORD, "tc"))
					.filter(|cell| {
						tree.descendant(*cell, Ns::WORD, "p").is_none()
					})
					.filter_map(|cell| {
						Some((cell, tree.element(cell)?.clone()))
					})
					.collect::<Vec<_>>()
			},
		);
		for (cell, named) in empty {
			world.spawn((
				OoxmlNode::Element(named.sibling(Ns::WORD, "p")),
				ChildOf(cell),
			));
		}
	}
}

/// A property an edit adds, its local name and the value of its `w:val`, a
/// toggle when none, ie `("b", None)`.
type Property = (&'static str, Option<SmolStr>);

/// A new text node: words with no element and no node of the file.
type NewText = (With<Value>, Without<Element>, Without<OoxmlNode>);

/// A properties element and the order its schema fixes for its children,
/// which Word holds a file to.
struct PropertyList {
	/// Its local name, ie `rPr`.
	local: &'static str,
	/// Its children's local names in order.
	order: &'static [&'static str],
}

impl PropertyList {
	/// A run's, `CT_RPr`.
	const RUN: Self = Self {
		local: "rPr",
		order: &[
			"rStyle",
			"rFonts",
			"b",
			"bCs",
			"i",
			"iCs",
			"caps",
			"smallCaps",
			"strike",
			"dstrike",
			"outline",
			"shadow",
			"emboss",
			"imprint",
			"noProof",
			"snapToGrid",
			"vanish",
			"webHidden",
			"color",
			"spacing",
			"w",
			"kern",
			"position",
			"sz",
			"szCs",
			"highlight",
			"u",
			"effect",
			"bdr",
			"shd",
			"fitText",
			"vertAlign",
			"rtl",
			"cs",
			"em",
			"lang",
			"eastAsianLayout",
			"specVanish",
			"oMath",
			"rPrChange",
		],
	};

	/// A table cell's, `CT_TcPr`.
	const CELL: Self = Self {
		local: "tcPr",
		order: &[
			"cnfStyle",
			"tcW",
			"gridSpan",
			"hMerge",
			"vMerge",
			"tcBorders",
			"shd",
			"noWrap",
			"tcMar",
			"textDirection",
			"tcFitText",
			"vAlign",
			"hideMark",
			"headers",
			"cellIns",
			"cellDel",
			"cellMerge",
			"tcPrChange",
		],
	};
}

/// A row an edit added to a table the file holds, `named` the table's
/// element, with the properties of its model row.
struct NewRow {
	row: Entity,
	named: OoxmlElement,
	properties: Option<Entity>,
}

/// A cell an edit added to a row written as a `w:tr`, `named` its table's
/// element, with the properties of its model cell and its `colspan`.
struct NewCell {
	cell: Entity,
	named: OoxmlElement,
	properties: Option<Entity>,
	span: u32,
	table: Entity,
}

/// How a new paragraph is written.
enum NewParagraph {
	/// With the properties of a paragraph of its own tag, its model.
	Like {
		model: Entity,
		properties: Option<Entity>,
	},
	/// A heading with no model, by its style.
	Heading(u8),
	/// Any other with no model, by its first sibling paragraph.
	Fallback {
		model: Option<Entity>,
		properties: Option<Entity>,
		is_item: bool,
	},
}

/// A link found under a Word file's root.
enum FoundLink {
	/// A `w:hyperlink` the file holds, by its relationship's `id`.
	Held {
		link: Entity,
		href: SmolStr,
		path: SmolStr,
		id: Option<SmolStr>,
	},
	/// One an edit added inside the paragraph `named`, `prefix` bound to the
	/// relationships namespace there, `model` the run properties of the
	/// file's nearest link.
	New {
		link: Entity,
		href: SmolStr,
		path: SmolStr,
		named: OoxmlElement,
		prefix: Option<SmolStr>,
		model: Option<Entity>,
	},
}

/// Every entity under a root by its place in document order.
struct DocumentOrder(HashMap<Entity, usize>);

impl DocumentOrder {
	fn new(tree: &OoxmlQuery, root: Entity) -> Self {
		Self(
			tree.descendants(root)
				.into_iter()
				.enumerate()
				.map(|(index, entity)| (entity, index))
				.collect(),
		)
	}

	/// Of `candidates`, the last before `entity`, else the first after it.
	fn nearest(
		&self,
		entity: Entity,
		candidates: impl IntoIterator<Item = Entity>,
	) -> Option<Entity> {
		let at = self.0.get(&entity).copied().unwrap_or_default();
		let (before, after): (Vec<_>, Vec<_>) = candidates
			.into_iter()
			.filter_map(|candidate| Some((*self.0.get(&candidate)?, candidate)))
			.partition(|(index, _)| *index < at);
		before
			.into_iter()
			.max_by_key(|(index, _)| *index)
			.or_else(|| after.into_iter().min_by_key(|(index, _)| *index))
			.map(|(_, candidate)| candidate)
	}
}

/// A deep copy of a part's subtree under `parent`, ie a model paragraph's
/// properties for a new one.
fn clone_nodes(world: &mut World, entity: Entity, parent: Entity) -> Entity {
	let copy = world.spawn(ChildOf(parent)).id();
	if let Some(node) = world.entity(entity).get::<OoxmlNode>().cloned() {
		world.entity_mut(copy).insert(node);
	}
	if let Some(value) = world.entity(entity).get::<Value>().cloned() {
		world.entity_mut(copy).insert(value);
	}
	let children = world
		.entity(entity)
		.get::<Children>()
		.map(|children| children.to_vec())
		.unwrap_or_default();
	for child in children {
		clone_nodes(world, child, copy);
	}
	copy
}

#[cfg(test)]
mod test {
	use super::super::renderer::test::*;
	use crate::prelude::*;
	use beet_core::prelude::*;

	fn document(bytes: &MediaBytes) -> String {
		part(bytes, "word/document.xml")
	}

	/// The element `tag` whose text holds `words`.
	fn find(world: &mut World, tag: &str, words: &str) -> Entity {
		let tagged = world
			.query::<(Entity, &Element)>()
			.iter(world)
			.filter(|(_, element)| element.tag() == tag)
			.map(|(entity, _)| entity)
			.collect::<Vec<_>>();
		world.with_state::<DocumentText, _>(|text| {
			tagged
				.into_iter()
				.find(|entity| text.text(*entity).contains(words))
				.unwrap()
		})
	}

	/// A new element `tag` with `attributes`, holding `text`, as an edit
	/// adds it, the last child of `parent`.
	fn add(
		world: &mut World,
		parent: Entity,
		tag: &str,
		attributes: &[(&str, &str)],
		text: &str,
	) -> Entity {
		let entity = world
			.spawn((Element::new(tag), ChildOf(parent), children![Value::str(
				text
			)]))
			.id();
		for (key, value) in attributes {
			world.spawn((
				AttributeOf::new(entity),
				Attribute::new(*key),
				Value::str(*value),
			));
		}
		entity
	}

	/// Parses `bytes`, runs `edit` on its tree, writes it back, and answers
	/// the written file with its markdown, read afresh.
	fn round_trip(
		bytes: MediaBytes,
		edit: impl FnOnce(&mut World, Entity),
	) -> (MediaBytes, String) {
		let (mut world, root) = parse(&bytes);
		edit(&mut world, root);
		let written = write(&mut world, root, MediaType::Docx);
		let (mut world, root) = parse(&written);
		let markdown = super::super::word::test::markdown(&mut world, root);
		(written, markdown)
	}

	/// A new heading takes the properties of a heading of its level, and
	/// with none its `Heading<n>` style.
	#[beet_core::test]
	fn writes_new_headings() {
		let (written, markdown) = round_trip(
			OoxmlFile::word(
				"<w:p><w:pPr><w:pStyle w:val=\"Heading2\"/><w:keepNext/></w:pPr><w:r><w:t>First</w:t></w:r></w:p>\
				 <w:p><w:r><w:t>Body</w:t></w:r></w:p>",
			)
			.unwrap(),
			|world, _| {
				let body = find(world, "p", "Body");
				let parent = world.entity(body).get::<ChildOf>().unwrap().parent();
				add(world, parent, "h2", &[], "Second");
			},
		);
		markdown.xpect_contains("## First\n\nBody\n\n## Second");
		document(&written).xpect_contains(
			"<w:p><w:pPr><w:pStyle w:val=\"Heading2\"/><w:keepNext/></w:pPr><w:r><w:t xml:space=\"preserve\">Second</w:t></w:r></w:p>",
		);

		let (written, markdown) = round_trip(
			OoxmlFile::word("<w:p><w:r><w:t>Body</w:t></w:r></w:p>").unwrap(),
			|world, _| {
				let body = find(world, "p", "Body");
				let parent =
					world.entity(body).get::<ChildOf>().unwrap().parent();
				add(world, parent, "h2", &[], "Second");
			},
		);
		markdown.xpect_contains("Body\n\n## Second");
		document(&written).xpect_contains(
			"<w:p><w:pPr><w:pStyle w:val=\"Heading2\"/></w:pPr><w:r><w:t xml:space=\"preserve\">Second</w:t></w:r></w:p>",
		);
	}

	/// A new list item joins its list by its neighbour's numbering.
	#[beet_core::test]
	fn joins_new_items_to_their_list() {
		round_trip(
			OoxmlFile::word(
				"<w:p><w:pPr><w:numPr><w:ilvl w:val=\"0\"/><w:numId w:val=\"1\"/></w:numPr></w:pPr><w:r><w:t>one</w:t></w:r></w:p>",
			)
			.unwrap(),
			|world, _| {
				let item = find(world, "li", "one");
				let list = world.entity(item).get::<ChildOf>().unwrap().parent();
				add(world, list, "li", &[], "two");
			},
		)
		.1
		.xpect_contains("- one\n- two\n");
	}

	/// New words inside new formatting elements are written with the run
	/// properties they say, each once and in schema order.
	#[beet_core::test]
	fn writes_new_inline_formatting() {
		let (written, markdown) = round_trip(
			OoxmlFile::word("<w:p><w:r><w:t>Plain</w:t></w:r></w:p>").unwrap(),
			|world, _| {
				let paragraph = find(world, "p", "Plain");
				for (tag, words) in
					[("strong", "bold"), ("em", "slanted"), ("mark", "lit")]
				{
					world.spawn((ChildOf(paragraph), Value::str(" ")));
					add(world, paragraph, tag, &[], words);
				}
				world.spawn((ChildOf(paragraph), Value::str(" ")));
				let strong = add(world, paragraph, "strong", &[], "");
				let em = add(world, strong, "em", &[], "");
				add(world, em, "b", &[], "both");
			},
		);
		markdown.xpect_contains(
			"Plain **bold** *slanted* <mark>lit</mark> ***both***",
		);
		document(&written)
			.xpect_contains("<w:r><w:rPr><w:b/></w:rPr><w:t xml:space=\"preserve\">bold</w:t></w:r>")
			.xpect_contains("<w:r><w:rPr><w:i/></w:rPr><w:t xml:space=\"preserve\">slanted</w:t></w:r>")
			.xpect_contains("<w:r><w:rPr><w:highlight w:val=\"yellow\"/></w:rPr><w:t xml:space=\"preserve\">lit</w:t></w:r>")
			.xpect_contains("<w:r><w:rPr><w:b/><w:i/></w:rPr><w:t xml:space=\"preserve\">both</w:t></w:r>");
	}

	/// New links in the main document and a header are hyperlinks of their
	/// part's relationships, one shared by two links to one address, in the
	/// look of the file's link; a changed link points at its new address.
	#[beet_core::test]
	fn writes_links_through_relationships() {
		let (written, markdown) = round_trip(
			OoxmlFile::word_with(
				"<w:p><w:hyperlink r:id=\"rIdOld\"><w:r><w:rPr><w:rStyle w:val=\"Hyperlink\"/></w:rPr><w:t>old</w:t></w:r></w:hyperlink></w:p>\
				 <w:p><w:r><w:t>See</w:t></w:r></w:p>",
				WordParts {
					header: Some("<w:p><w:r><w:t>Top</w:t></w:r></w:p>"),
					links: &[("rIdOld", "https://old.example/")],
					..default()
				},
			)
			.unwrap(),
			|world, _| {
				let see = find(world, "p", "See");
				for words in ["one", "two"] {
					world.spawn((ChildOf(see), Value::str(" ")));
					add(world, see, "a", &[("href", "https://a.example/")], words);
				}
				let top = find(world, "p", "Top");
				world.spawn((ChildOf(top), Value::str(" ")));
				add(world, top, "a", &[("href", "https://b.example/")], "head");
				let old = find(world, "a", "old");
				let href = world.with_state::<ElementQuery, _>(|elements| {
					elements
						.get(old)
						.unwrap()
						.attribute_with_entity("href")
						.unwrap()
						.0
				});
				world
					.entity_mut(href)
					.insert(Value::str("https://new.example/"));
			},
		);
		markdown
			.xpect_contains("Top [head](https://b.example/)")
			.xpect_contains("[old](https://new.example/)")
			.xpect_contains(
				"See [one](https://a.example/) [two](https://a.example/)",
			);
		let file = OoxmlFile::open(&written).unwrap();
		let main = file.part_at("word/document.xml").unwrap();
		let mut targets =
			file.hyperlinks(&main).into_values().collect::<Vec<_>>();
		targets.sort();
		// the old address kept, since another link may share it
		targets.xpect_eq(vec![
			SmolStr::new("https://a.example/"),
			"https://new.example/".into(),
			"https://old.example/".into(),
		]);
		let link_run = "<w:r><w:rPr><w:rStyle w:val=\"Hyperlink\"/></w:rPr><w:t xml:space=\"preserve\">";
		document(&written)
			.xpect_contains(format!("{link_run}one</w:t>"))
			.xpect_contains(format!("{link_run}two</w:t>"));
		let header = file
			.parts()
			.into_iter()
			.find(|part| {
				matches!(part, ooxmlsdk::parts::PartRef::HeaderPart(_))
			})
			.unwrap();
		let header_path = file.path(&header).unwrap();
		// the header's root binds no `r:`, so the link binds it
		part(&written, &header_path).xpect_contains(format!(
			"<w:hyperlink xmlns:r=\"{}\" r:id=\"",
			OoxmlNamespace::RELATIONSHIPS
		));
		file.hyperlinks(&header)
			.into_values()
			.collect::<Vec<_>>()
			.xpect_eq(vec![SmolStr::new("https://b.example/")]);
	}

	/// A new table and a new link to a place in the file are written as
	/// their text, and named as dropped.
	#[beet_core::test]
	fn writes_what_does_not_reverse_as_text() {
		let bytes =
			OoxmlFile::word("<w:p><w:r><w:t>Body</w:t></w:r></w:p>").unwrap();
		let edit = |world: &mut World, _: Entity| {
			let body = find(world, "p", "Body");
			add(world, body, "a", &[("href", "#place")], " here");
			let parent = world.entity(body).get::<ChildOf>().unwrap().parent();
			let table = add(world, parent, "table", &[], "");
			let row = add(world, table, "tr", &[], "");
			add(world, row, "td", &[], "cell");
		};
		let (mut world, root) = parse(&bytes);
		edit(&mut world, root);
		super::WordWriter::dropped(&mut world, root)
			.xpect_eq(vec!["a href=\"#..\"", "table"]);
		let (written, markdown) = round_trip(bytes, edit);
		markdown
			.xpect_contains("Body here\n\ncell")
			.xnot()
			.xpect_contains("[here]");
		document(&written).xnot().xpect_contains("w:tbl");
	}

	/// Moves `entity` to `index` among `parent`'s children.
	fn place(world: &mut World, parent: Entity, index: usize, entity: Entity) {
		world.entity_mut(entity).remove::<ChildOf>();
		world.entity_mut(parent).insert_child(index, entity);
	}

	/// The first table's cells with their positions and texts, `A1 Name`.
	fn addressed(world: &mut World) -> Vec<String> {
		let table = world
			.query::<(Entity, &Element)>()
			.iter(world)
			.find(|(_, element)| element.tag() == "table")
			.map(|(entity, _)| entity)
			.unwrap();
		world.with_state::<(TableGrid, DocumentText), _>(|(grid, text)| {
			grid.cells(table)
				.into_iter()
				.map(|(cell, position)| {
					format!("{position} {}", text.own_text(cell).trim())
				})
				.collect()
		})
	}

	/// Rows and cells added to a table the file holds are its own: a row in
	/// its neighbour's properties but never a repeated header, a cell in its
	/// column's but never merged down, spanning as it says, and the grid
	/// widened to the widest row.
	#[beet_core::test]
	fn writes_rows_and_cells_into_a_held_table() {
		let (written, _) = round_trip(
			OoxmlFile::word(
				"<w:tbl><w:tblGrid><w:gridCol w:w=\"2000\"/><w:gridCol w:w=\"3000\"/></w:tblGrid>\
				 <w:tr><w:trPr><w:tblHeader/></w:trPr>\
				 <w:tc><w:tcPr><w:tcW w:w=\"2000\" w:type=\"dxa\"/></w:tcPr><w:p><w:r><w:t>Name</w:t></w:r></w:p></w:tc>\
				 <w:tc><w:tcPr><w:tcW w:w=\"3000\" w:type=\"dxa\"/><w:vMerge w:val=\"restart\"/></w:tcPr><w:p><w:r><w:t>Acme</w:t></w:r></w:p></w:tc>\
				 </w:tr></w:tbl>",
			)
			.unwrap(),
			|world, _| {
				let first = find(world, "tr", "Name");
				let table = world.entity(first).get::<ChildOf>().unwrap().parent();
				add(world, first, "td", &[], "Extra");
				let second = add(world, table, "tr", &[], "");
				add(world, second, "td", &[], "Phone");
				add(world, second, "td", &[], "555");
				let third = add(world, table, "tr", &[], "");
				add(world, third, "td", &[("colspan", "2")], "Wide");
			},
		);
		let (mut world, _) = parse(&written);
		addressed(&mut world).xpect_eq(vec![
			"A1 Name".to_string(),
			"B1 Acme".into(),
			"C1 Extra".into(),
			"A2 Phone".into(),
			"B2 555".into(),
			"A3 Wide".into(),
		]);
		let width = |width: u32| {
			format!("<w:tcPr><w:tcW w:w=\"{width}\" w:type=\"dxa\"/></w:tcPr>")
		};
		document(&written)
			// the header row's properties left behind, the merge too
			.xpect_contains(format!("</w:tr><w:tr><w:tc>{}<w:p>", width(2000)))
			.xpect_contains(format!("{}<w:p><w:r><w:t xml:space=\"preserve\">555", width(3000)))
			.xpect_contains(format!("{}<w:p><w:r><w:t xml:space=\"preserve\">Extra", width(3000)))
			.xpect_contains("<w:tcPr><w:tcW w:w=\"2000\" w:type=\"dxa\"/><w:gridSpan w:val=\"2\"/></w:tcPr>")
			.xpect_contains("<w:tblGrid><w:gridCol w:w=\"2000\"/><w:gridCol w:w=\"3000\"/><w:gridCol w:w=\"3000\"/></w:tblGrid>");
	}

	/// Words where a paragraph belongs are written in a new one, a run of
	/// whitespace alone in none; a new break and a `pre`'s lines are breaks.
	#[beet_core::test]
	fn encloses_loose_words_in_paragraphs() {
		let (written, markdown) = round_trip(
			OoxmlFile::word("<w:p><w:r><w:t>Body</w:t></w:r></w:p>").unwrap(),
			|world, _| {
				let body = find(world, "p", "Body");
				world.spawn((Element::new("br"), ChildOf(body)));
				world.spawn((Value::str("next"), ChildOf(body)));
				let parent =
					world.entity(body).get::<ChildOf>().unwrap().parent();
				world.spawn((Value::str("loose "), ChildOf(parent)));
				add(world, parent, "strong", &[], "words");
				world.spawn((Value::str("\n  "), ChildOf(parent)));
				add(world, parent, "pre", &[], "one\ntwo\n");
			},
		);
		markdown.xpect_contains("loose **words**");
		let run = |words: &str| {
			format!("<w:r><w:t xml:space=\"preserve\">{words}</w:t></w:r>")
		};
		let line_break = "<w:r><w:br/></w:r>";
		document(&written)
			.xpect_contains(format!("<w:t>Body</w:t></w:r>{line_break}{}</w:p>", run("next")))
			// the edge whitespace left between the paragraphs, as XML ignores
			.xpect_contains(format!(
				"<w:p>{}<w:r><w:rPr><w:b/></w:rPr><w:t xml:space=\"preserve\">words</w:t></w:r></w:p>\n  <w:p>",
				run("loose "),
			))
			.xpect_contains(format!(
				"<w:p>{}{line_break}{}</w:p>",
				run("one"),
				run("two")
			));
	}

	/// New words or a break inside a run the file holds get a run of their
	/// own in its look, between what came before and what came after.
	#[beet_core::test]
	fn lifts_new_words_out_of_a_held_run() {
		let (written, _) = round_trip(
			OoxmlFile::word(
				"<w:p><w:r><w:rPr><w:i/></w:rPr><w:t>Hello</w:t><w:t xml:space=\"preserve\"> world</w:t></w:r></w:p>\
				 <w:p><w:r><w:rPr><w:b/><w:i/></w:rPr><w:t>Both</w:t></w:r></w:p>",
			)
			.unwrap(),
			|world, _| {
				let run = find(world, "em", "Hello");
				let strong = add(world, run, "strong", &[], " big");
				place(world, run, 2, strong);
				// lifted again from the copy holding what followed
				world.spawn((Element::new("br"), ChildOf(run)));
				// inside the reader's own `<em>` within a bold, italic run
				let both = find(world, "strong", "Both");
				let em = find(world, "em", "Both");
				world.entity(em).get::<ChildOf>().unwrap().parent().xpect_eq(both);
				world.spawn((Value::str(" more"), ChildOf(em)));
			},
		);
		document(&written)
			.xpect_contains(
				"<w:r><w:rPr><w:i/></w:rPr><w:t>Hello</w:t></w:r>\
				 <w:r><w:rPr><w:b/><w:i/></w:rPr><w:t xml:space=\"preserve\"> big</w:t></w:r>\
				 <w:r><w:rPr><w:i/></w:rPr><w:t xml:space=\"preserve\"> world</w:t></w:r>\
				 <w:r><w:br/></w:r></w:p>",
			)
			.xpect_contains(
				"<w:r><w:rPr><w:b/><w:i/></w:rPr><w:t>Both</w:t></w:r>\
				 <w:r><w:rPr><w:b/><w:i/></w:rPr><w:t xml:space=\"preserve\"> more</w:t></w:r>",
			);
	}
}
