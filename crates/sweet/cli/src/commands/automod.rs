use beet::exports::notify::EventKind;
use beet::exports::notify::event::ModifyKind;
use beet::exports::notify::event::RenameMode;
use beet::prelude::*;
use clap::Parser;
use quote::quote;
use rapidhash::RapidHashMap;
use std::path::Path;
use std::path::PathBuf;
use syn::File;
use syn::Ident;
use syn::ItemMod;
use syn::ItemUse;
use syn::UseTree;

#[derive(Debug, Default, Clone, Parser)]
#[command(name = "mod")]
pub struct AutoMod {
	#[command(flatten)]
	pub watcher: FsWatcher,

	#[arg(short, long)]
	pub quiet: bool,
}

/// Returns whether a change was made
#[derive(PartialEq)]
enum DidMutate {
	No,
	/// For printing
	Yes {
		action: String,
		path: PathBuf,
	},
}


impl AutoMod {
	pub async fn run(mut self) -> Result {
		self.watcher.assert_path_exists()?;
		if !self.quiet {
			println!(
				"🤘 sweet as 🤘\nWatching for file changes in {}",
				self.watcher.cwd.canonicalize()?.display()
			);
		}

		self.watcher.filter = self
			.watcher
			.filter
			.with_exclude("*/tests/*")
			.with_exclude("*/examples/*")
			.with_exclude("*/bin/*")
			.with_exclude("**/mod.rs")
			.with_exclude("**/lib.rs")
			.with_exclude("**/main.rs")
			.with_include("**/*.rs");
		let mut rx = self.watcher.watch()?;
		while let Some(ev) = rx.recv().await? {
			let mut files = ModFiles::default();
			let any_mutated = ev
				.iter()
				.map(|e| self.handle_event(&mut files, e))
				.collect::<Result<Vec<_>>>()?
				.into_iter()
				.filter_map(|r| match r {
					DidMutate::No => None,
					DidMutate::Yes { action, path } => {
						if !self.quiet {
							println!(
								"AutoMod: {action} {}",
								path_ext::relative(&path)
									.unwrap_or(&path)
									.display(),
							);
						}
						Some(())
					}
				})
				.next()
				.is_some();
			if any_mutated {
				files.write_all()?;
			}
		}
		Ok(())
	}


	fn handle_event(
		&self,
		files: &mut ModFiles,
		e: &WatchEvent,
	) -> Result<DidMutate> {
		enum Step {
			Insert,
			Remove,
		}

		// let (parent_mod, mod_file) = Self::insert_mod(&e.path)?;
		// self.write_file("insert", &e.path, parent_mod, mod_file)?;

		let step = match e.kind {
			EventKind::Create(_)
			| EventKind::Modify(ModifyKind::Name(RenameMode::To)) => Step::Insert,
			EventKind::Remove(_)
			| EventKind::Modify(ModifyKind::Name(RenameMode::From)) => Step::Remove,
			EventKind::Modify(ModifyKind::Name(_))
			| EventKind::Modify(ModifyKind::Data(_)) => {
				if e.path.exists() {
					Step::Insert
				} else {
					Step::Remove
				}
			}
			_ => {
				return Ok(DidMutate::No);
			}
		};

		let file_meta = FileMeta::new(&e.path)?;
		let file = files.get_mut(&file_meta.parent_mod)?;
		match step {
			Step::Insert => Self::insert_mod(file, file_meta),
			Step::Remove => Self::remove_mod(file, file_meta),
		}
	}

	/// Load the parents `mod.rs` or `lib.rs` file and insert a new module
	fn insert_mod(
		mod_file: &mut File,
		FileMeta {
			is_lib_dir,
			file_stem,
			mod_ident,
			event_path,
			..
		}: FileMeta,
	) -> Result<DidMutate> {
		for item in &mut mod_file.items {
			if let syn::Item::Mod(m) = item {
				if m.ident == file_stem {
					// module already exists, nothing to do here
					return Ok(DidMutate::No);
				}
			}
		}

		let vis = if is_lib_dir {
			quote! {pub}
		} else {
			Default::default()
		};


		let insert_pos = mod_file
			.items
			.iter()
			.position(|item| matches!(item, syn::Item::Mod(_)))
			.unwrap_or(mod_file.items.len());

		let mod_def: ItemMod = syn::parse_quote!(#vis mod #mod_ident;);
		mod_file.items.insert(insert_pos, mod_def.into());

		if is_lib_dir {
			// export in prelude
			for item in &mut mod_file.items {
				if let syn::Item::Mod(m) = item {
					if m.ident == "prelude" {
						if let Some(content) = m.content.as_mut() {
							content.1.push(
								syn::parse_quote!(pub use crate::#mod_ident::*;),
							);
						} else {
							m.content =
								Some((syn::token::Brace::default(), vec![
									syn::parse_quote!(pub use crate::#mod_ident::*;),
								]));
						}
						break;
					}
				}
			}
		} else {
			// export at root
			mod_file.items.insert(
				insert_pos + 1,
				syn::parse_quote!(pub use #mod_ident::*;),
			);
		}

		Ok(DidMutate::Yes {
			action: "insert".into(),
			path: event_path.to_path_buf(),
		})
	}

	fn remove_mod(
		mod_file: &mut File,
		FileMeta {
			is_lib_dir,
			file_stem,
			mod_ident,
			event_path,
			..
		}: FileMeta,
	) -> Result<DidMutate> {
		let mut did_mutate = false;
		mod_file.items.retain(|item| {
			if let syn::Item::Mod(m) = item {
				if m.ident == file_stem {
					did_mutate = true;
					return false;
				}
			}
			true
		});

		// Remove the re-export
		if is_lib_dir {
			// Remove from prelude
			for item in &mut mod_file.items {
				if let syn::Item::Mod(m) = item {
					if m.ident == "prelude" {
						if let Some(content) = m.content.as_mut() {
							content.1.retain(|item| {
								if let syn::Item::Use(use_item) = item {
									if let Some(last) = use_item_ident(use_item)
									{
										if last == &mod_ident {
											did_mutate = true;
											return false;
										}
									}
								}
								true
							});
						}
						break;
					}
				}
			}
		} else {
			// Remove re-export at root
			mod_file.items.retain(|item| {
				if let syn::Item::Use(use_item) = item {
					if let Some(last) = use_item_ident(use_item) {
						if last == &mod_ident {
							did_mutate = true;
							return false;
						}
					}
				}
				true
			});
		}

		Ok(match did_mutate {
			true => DidMutate::Yes {
				action: "remove".into(),
				path: event_path.to_path_buf(),
			},
			false => DidMutate::No,
		})
	}
}
/// find the first part of an ident, skiping `crate`, `super` or `self`
fn use_item_ident(use_item: &ItemUse) -> Option<&Ident> {
	const SKIP: [&str; 3] = ["crate", "super", "self"];
	match &use_item.tree {
		UseTree::Path(use_path) => {
			if SKIP.contains(&use_path.ident.to_string().as_str()) {
				match &*use_path.tree {
					UseTree::Path(use_path) => {
						return Some(&use_path.ident);
					}
					UseTree::Name(use_name) => {
						return Some(&use_name.ident);
					}
					_ => {}
				}
			} else {
				return Some(&use_path.ident);
			}
		}
		_ => {}
	}
	None
}

#[derive(Default, Clone)]
struct ModFiles {
	map: RapidHashMap<PathBuf, File>,
}

impl ModFiles {
	/// Get a mutable reference to the file at the given path.
	/// If it doesnt exist, an empty file is created, and will be
	/// written to disk on [`ModFiles::write_all`].
	pub fn get_mut(&mut self, path: impl AsRef<Path>) -> Result<&mut File> {
		let path = path.as_ref();
		if !self.map.contains_key(path) {
			// if it doesnt exist create an empty file
			let file = fs_ext::read_to_string(path).unwrap_or_default();
			let file = syn::parse_file(&file)?;
			self.map.insert(path.to_path_buf(), file);
		}
		Ok(self.map.get_mut(path).unwrap())
	}
	pub fn write_all(&self) -> Result {
		// TODO only perform write if hash changed
		for (path, file) in &self.map {
			let file = prettyplease::unparse(file);
			fs_ext::write(path, &file)?;
			println!(
				"AutoMod: write  {}",
				path_ext::relative(path).unwrap_or(path).display()
			);
		}
		Ok(())
	}
}

struct FileMeta<'a> {
	pub is_lib_dir: bool,
	pub parent_mod: PathBuf,
	pub file_stem: String,
	#[allow(dead_code)]
	pub event_path: &'a Path,
	pub mod_ident: syn::Ident,
}

impl<'a> FileMeta<'a> {
	/// Returns either `lib.rs` or `mod.rs` for the given path's parent
	fn new(event_path: &'a Path) -> Result<Self> {
		let Some(parent) = event_path.parent() else {
			bevybail!("No parent found for path {}", event_path.display());
		};
		let is_lib_dir =
			parent.file_name().map(|f| f == "src").unwrap_or(false);
		let parent_mod = if is_lib_dir {
			parent.join("lib.rs")
		} else {
			parent.join("mod.rs")
		};
		let Some(file_stem) = event_path
			.file_stem()
			.map(|s| s.to_string_lossy().to_string())
		else {
			bevybail!("No file stem found for path {}", event_path.display());
		};

		let mod_ident =
			syn::Ident::new(&file_stem, proc_macro2::Span::call_site());

		Ok(Self {
			event_path,
			is_lib_dir,
			parent_mod,
			file_stem,
			mod_ident,
		})
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use sweet::prelude::*;

	#[test]
	fn insert_works() {
		fn insert(ws_path: impl AsRef<Path>) -> Result<String> {
			let abs =
				AbsPathBuf::new(fs_ext::workspace_root().join(ws_path.as_ref()))
					.unwrap();
			let file_meta = FileMeta::new(abs.as_ref())?;
			let file = fs_ext::read_to_string(&file_meta.parent_mod)?;
			let mut file = syn::parse_file(&file)?;
			AutoMod::insert_mod(&mut file, file_meta)?;
			let file = prettyplease::unparse(&file);
			Ok(file)
		}

		let insert_lib = insert("crates/sweet/cli/src/foo.rs").unwrap();
		(&insert_lib).xpect_contains("pub mod foo;");
		(&insert_lib).xpect_contains("pub use crate::foo::*;");

		let insert_mod =
			insert("crates/sweet/cli/src/commands/foo.rs").unwrap();
		(&insert_mod).xpect_contains("mod foo;");
		(&insert_mod).xpect_contains("pub use foo::*;");
	}
	#[test]
	fn remove_works() {
		fn remove(ws_path: impl AsRef<Path>) -> Result<String> {
			let abs =
				AbsPathBuf::new(fs_ext::workspace_root().join(ws_path.as_ref()))
					.unwrap();
			let file_meta = FileMeta::new(abs.as_ref())?;
			let file = fs_ext::read_to_string(&file_meta.parent_mod)?;
			let mut file = syn::parse_file(&file)?;
			AutoMod::remove_mod(&mut file, file_meta)?;
			let file = prettyplease::unparse(&file);
			Ok(file)
		}

		let remove_lib = remove("crates/sweet/cli/src/automod").unwrap();
		(&remove_lib).xnot().xpect_contains("pub mod automod;");
		(&remove_lib)
			.xnot()
			.xpect_contains("pub use crate::automod::*;");


		let remove_mod =
			remove("crates/sweet/cli/src/commands/automod.rs").unwrap();
		(&remove_mod).xnot().xpect_contains("pub mod automod;");
		(&remove_mod).xnot().xpect_contains("pub use automod::*;");
	}
}
