#[cfg(target_arch = "wasm32")]
use crate::prelude::js_runtime;
use ::test::TestDesc;
use anyhow::Result;
use backtrace::BacktraceFrame;
use colorize::*;
use std::panic::PanicHookInfo;
use std::path::Path;
use std::path::PathBuf;

pub struct BacktraceLocation {
	/// the absolute file location
	pub cwd_path: PathBuf,
	/// the file location relative to the workspace root
	// pub path_workspace: PathBuf,
	pub line_no: usize,
	pub col_no: usize,
}

impl BacktraceLocation {
	/// The input path from test desc, panic info, or backtrace frame
	/// is relative to cwd(), we resolve relative to workspace root
	/// for terminal links and cleaner output
	pub fn with_cwd(
		cwd_path: impl AsRef<Path>,
		line_no: usize,
		col_no: usize,
	) -> Self {
		Self {
			cwd_path: cwd_path.as_ref().to_path_buf(),
			line_no,
			col_no,
		}
	}

	/// In wasm we dont get a backtrace so instead use the test entrypoint
	pub fn from_test_desc(desc: &TestDesc) -> Self {
		Self::with_cwd(&desc.source_file, desc.start_line, desc.start_col)
	}
	/// Use location of the panic, will fall back to desc if no location is found
	pub fn from_panic_info(info: &PanicHookInfo, desc: &TestDesc) -> Self {
		if let Some(location) = info.location() {
			Self::with_cwd(
				location.file(),
				location.line() as usize,
				location.column() as usize,
			)
		} else {
			Self::from_test_desc(desc)
		}
	}

	pub fn from_unresolved_frame(
		frame: &BacktraceFrame,
	) -> anyhow::Result<Self> {
		let mut frame = frame.to_owned().clone();
		frame.resolve();

		let symbol = frame
			.symbols()
			.get(0)
			.ok_or_else(|| anyhow::anyhow!("No symbols"))?;
		let file = symbol
			.filename()
			.ok_or_else(|| anyhow::anyhow!("Bactrace has no file"))?;

		let line_no = symbol.lineno().unwrap_or_default() as usize;
		let col_no = symbol.colno().unwrap_or_default() as usize;

		Ok(Self::with_cwd(file, line_no, col_no))
	}
	///
	/// Efficiently resolves workspace relative path by
	/// passing in the workspace root via [`BacktraceLocation::cwd_root`]
	/// for a given error `it failed!` format like so:
	///
	/// ```ignore
	/// at path/to/file_name.rs:1:2
	/// ```
	pub fn stack_line_string(&self, cwd_root: &Path) -> String {
		let prefix = String::from("at").faint();

		let workspace_path = self
			.cwd_path
			.strip_prefix(&cwd_root)
			.unwrap_or(&self.cwd_path)
			.to_string_lossy()
			.to_string()
			.cyan();
		let line_loc =
			String::from(format!(":{}:{}", self.line_no, self.col_no)).faint();

		format!("{} {}{}", prefix, workspace_path, line_loc)
	}

	/// Number of lines before and after the panic of the file to load
	pub const LINE_CONTEXT_SIZE: usize = 2;


	/// Return the panicking line surronded by SELF::LINE_CONTEXT_SIZE lines
	/// of context
	/// Also return a stack trace of at least the first frame,
	/// and loosely matching RUST_BACKTRACE=1, with some extra opinions suitable for sweet
	/// # Errors
	/// This function will return an error if the file cannot be read
	pub fn file_context(&self) -> Result<String> {
		let cwd_root = Self::cwd_root();
		let abs_path = cwd_root.join(&self.cwd_path);

		let file = read_file(&abs_path)?;
		let lines: Vec<&str> = file.split("\n").collect();
		//line number is one-indexed
		let start = usize::max(
			0,
			self.line_no.saturating_sub(Self::LINE_CONTEXT_SIZE + 1),
		);
		let end =
			usize::min(lines.len() - 1, self.line_no + Self::LINE_CONTEXT_SIZE);

		let mut output = String::new();

		for i in start..end {
			let curr_line_no = i + 1;
			let is_err_line = curr_line_no == self.line_no;
			let prefix =
				String::from(if is_err_line { ">" } else { " " }).red();

			let buffer = line_number_buffer(curr_line_no);
			let line_prefix =
				String::from(format!("{}{}|", curr_line_no, buffer)).faint();
			let full_prefix = format!("{} {}", prefix, line_prefix);
			// let prefix_len = 6;
			output.push_str(&full_prefix);
			output.push_str(lines[i]);
			output.push('\n');
			if is_err_line {
				//TODO string length
				output.push_str(
					&format!("{}|", " ".repeat(2 + LINE_BUFFER_LEN)).faint(),
				);
				output.push_str(&" ".repeat(self.col_no));
				output.push_str(&String::from("^").red().as_str());
				output.push('\n');
			}
		}

		let mut stack_locations = vec![self.stack_line_string(&cwd_root)];

		if std::env::var("RUST_BACKTRACE").unwrap_or_default() == "full" {
			stack_locations.extend(
				backtrace::Backtrace::new().frames().into_iter().filter_map(
					|frame| {
						Self::from_unresolved_frame(&frame)
							.map(|loc| loc.stack_line_string(&cwd_root))
							.ok()
					},
				),
			);
		} else if std::env::var("RUST_BACKTRACE").unwrap_or_default() == "1" {
			let bt = backtrace::Backtrace::new();
			let locations = bt
				.frames()
				.iter()
				.filter(|frame| {
					let sym = match frame.symbols().first() {
						Some(s) => s,
						None => return false,
					};

					if sym.filename().map_or(true, |f| {
						let s = f.to_string_lossy();
						s.contains(".cargo/registry/src/")
							|| s.contains(".rustup/toolchains/")
							|| s.contains("library/std/src/")
							|| s.contains("library/alloc/src/")
							|| s.contains("sweet_test/src/")
					}) {
						return false;
					}
					let name = match sym.name() {
						Some(n) => n.to_string(),
						None => return false,
					};

					!name.starts_with("std::")
						&& !name.starts_with("core::")
						&& !name.starts_with("backtrace::")
						&& !name.starts_with("rayon::")
				})
				.filter_map(|frame| {
					Self::from_unresolved_frame(&frame)
						.map(|loc| loc.stack_line_string(&cwd_root))
						.ok()
				});
			stack_locations.extend(locations);
		}

		output.push('\n');
		output.push_str(&stack_locations.join("\n"));

		Ok(output)
	}
	/// The root of cwd:
	/// - for `cargo test` this is the workspace root
	/// - for `cargo test - my_pkg` this uses $SWEET_ROOT to resolve package root
	/// 1. Prefix the path with $SWEET_ROOT if it exists,
	/// 2. otherwise use [FsExt::workspace_root]
	pub fn cwd_root() -> PathBuf {
		#[cfg(not(target_arch = "wasm32"))]
		return std::env::var("SWEET_ROOT")
			.map(PathBuf::from)
			.unwrap_or_else(|_| sweet_utils::prelude::FsExt::workspace_root());
		#[cfg(target_arch = "wasm32")]
		return js_runtime::sweet_root()
			.map(PathBuf::from)
			.unwrap_or_default();
	}
}


/// Read a file either from fs or wasm runtime,
/// printing helpful error if couldnt read
fn read_file(path: &Path) -> Result<String> {
	let bail = |cwd: &str| {
		let sweet_root = std::env::var("SWEET_ROOT");
		anyhow::anyhow!(
			"Failed to read file:\ncwd:\t{}\npath:\t{}\nSWEET_ROOT: {:?}\n{CONTEXT}",
			cwd,
			&path.display(),
			sweet_root
		)
	};

	const CONTEXT: &str = r#"
This can happen when working with workspaces and the sweet root has not been set.
(This setting is required because rust does not have a CARGO_WORKSPACE_DIR)

Please configure the following:

``` .cargo/config.toml

[env]
SWEET_ROOT = { value = "", relative = true }

```
"#;

	#[cfg(target_arch = "wasm32")]
	let file = js_runtime::read_file(&path.to_string_lossy().to_string())
		.ok_or_else(|| bail(&js_runtime::cwd()))?;
	#[cfg(not(target_arch = "wasm32"))]
	let file = sweet_utils::prelude::ReadFile::to_string(path).map_err(|_| {
		bail(
			&std::env::current_dir()
				.unwrap_or_default()
				.display()
				.to_string(),
		)
	})?;

	Ok(file)
}


const LINE_BUFFER_LEN: usize = 3;

fn line_number_buffer(line_no: usize) -> String {
	let line_no = line_no.to_string();
	let digits = line_no.len();
	let len = LINE_BUFFER_LEN.saturating_sub(digits);
	" ".repeat(len)
}
