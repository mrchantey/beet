use anyhow::Result;
use clap::Parser;
use std::fs;
use std::process::Command;

/// Measure the compilation time for the assert! macro
///
/// For context of an average large project:
///
/// `egrep -r "assert[!_]" . | wc -l`
///
/// bevy: 7,000
/// wasm-bindgen: 3,000
/// rust: 50,000
///
/// ## Expect
/// 10 lines of 'expect' comilied in 0.53s, each line added 53.00ms
/// 100 lines of 'expect' comilied in 0.47s, each line added 4.70ms
/// 1000 lines of 'expect' comilied in 0.49s, each line added 0.49ms
/// 2000 lines of 'expect' comilied in 0.50s, each line added 0.25ms
/// 3000 lines of 'expect' comilied in 0.53s, each line added 0.18ms
/// 5000 lines of 'expect' comilied in 0.56s, each line added 0.11ms
/// 10000 lines of 'expect' comilied in 0.70s, each line added 0.07ms * consistency starts here
/// 100000 lines of 'expect' comilied in 5.37s, each line added 0.05ms
/// 20000 lines of 'expect' comilied in 1.06s, each line added 0.05ms
/// 500000 lines of 'expect' comilied in 44.00s, each line added 0.09ms
///
/// ## Assert
///
/// 10 lines of 'assert' comilied in 0.21s, each line added 21.00ms
/// 100 lines of 'assert' comilied in 0.23s, each line added 2.30ms
/// 1000 lines of 'assert' comilied in 1.54s, each line added 1.54ms * smallest
/// 2000 lines of 'assert' comilied in 4.92s, each line added 2.46ms
/// 3000 lines of 'assert' comilied in 11.61s, each line added 3.87ms
/// 5000 lines of 'assert' comilied in 26.96s, each line added 5.39ms * consistency starts here
/// 10000 lines of 'assert' comilied in 55.00s, each line added 5.50ms
/// 20000 lines of 'expect' comilied in 1.06s, each line added 0.05ms * this is incorrect, it actually took 10 mins
/// 100000... no way dude
#[derive(Debug, Parser)]
pub struct BenchAssert {
	#[arg(long, default_value_t = 1000)] // 1000 is the most gracious
	iterations: usize,
	#[arg(long)]
	expect_only: bool,
	#[arg(long)]
	assert_only: bool,
	/// no detectable difference
	#[arg(long)]
	release: bool,
	/// no detectable difference
	#[arg(long)]
	run: bool,
}

const BENCH_DIR: &str = "./tests";

impl BenchAssert {
	pub fn run(self) -> Result<()> {
		fs::create_dir_all(BENCH_DIR)?;

		if self.expect_only {
			self.run_expect()?;
		} else if self.assert_only {
			self.run_assert()?;
		} else {
			self.run_expect()?;
			self.run_assert()?;
		}

		Ok(())
	}


	fn run_assert(&self) -> Result<()> {
		self.create_iter_file(ASSERT_FILE_PATH, ASSERT_TEMPLATE, |i| {
			format!("\tassert_eq!({},{});\n", i, i)
		})?;
		self.bench_compile("assert")?;
		if self.run {
			self.bench_run("assert")?;
		}
		Ok(())
	}
	fn run_expect(&self) -> Result<()> {
		self.create_iter_file(EXPECT_FILE_PATH, EXPECT_TEMPLATE, |i| {
			format!("\texpect({},{});\n", i, i)
		})?;
		self.bench_compile("expect")?;
		if self.run {
			self.bench_run("expect")?;
		}
		Ok(())
	}

	fn create_iter_file(
		&self,
		file_path: &str,
		file_template: &str,
		mk_str: impl Fn(usize) -> String,
	) -> Result<()> {
		let mut iterations = String::new();
		for i in 0..self.iterations {
			iterations.push_str(&mk_str(i));
		}

		let output =
			String::from(file_template).replace("__iterations__", &iterations);

		fs::write(file_path, output)?;
		Ok(())
	}


	fn bench_compile(&self, test_name: &str) -> Result<()> {
		// let path = path::Path::new(BENCH_DIR).join(test_name);
		let mut command = Command::new("cargo");
		command.arg("build").arg("--test").arg(test_name);
		if self.release {
			command.arg("--release");
		}
		let output = command.output()?;

		let stderr = String::from_utf8_lossy(&output.stderr);

		let duration = stderr
			.lines()
			.find(|line| line.contains("Finished"))
			.expect("line not found")
			.split(" ")
			.last()
			.unwrap()
			.replace("s", "")
			.parse::<f64>()
			.unwrap();

		let time_per_iter = (duration / self.iterations as f64) * 1000.;

		println!(
			"{} lines of '{}' comilied in {:.2}s, each line added {:.2}ms",
			self.iterations, test_name, duration, time_per_iter
		);
		Ok(())
	}

	fn bench_run(&self, test_name: &str) -> Result<()> {
		let output = Command::new("cargo")
			.arg("test")
			.arg("--test")
			.arg(test_name)
			.arg("--")
			.arg("--nocapture")
			.output()?;
		let output = String::from_utf8_lossy(&output.stdout);
		println!("{}", output);

		let duration = output
			.lines()
			.find(|line| line.contains("__"))
			.and_then(|line| line.split("__").nth(1))
			.and_then(|num| num.parse::<f64>().ok())
			.expect("Failed to find and parse number");

		let time_per_iter = (duration / self.iterations as f64) * 1000.;

		println!(
			"{} lines of '{}' ran in {:.2}s, each line added {:.2}ms",
			self.iterations, test_name, duration, time_per_iter
		);

		Ok(())
	}
}

const ASSERT_FILE_PATH: &str = "./tests/assert.rs";
const ASSERT_TEMPLATE: &str = r#"
	use std::time::Instant;
	#[test]	
	fn main(){
  	let start = Instant::now();
__iterations__
		println!("__{:.2}__", start.elapsed().as_secs_f32());
}
"#;
const EXPECT_FILE_PATH: &str = "./tests/expect.rs";
const EXPECT_TEMPLATE: &str = r#"
	use std::time::Instant;
	#[test]
	fn main(){
  	let start = Instant::now();
__iterations__
		println!("__{:.2}__", start.elapsed().as_secs_f32());
	}

	fn expect(a: i32, b: i32) {
		if a != b {
			panic!("Expected {} but got {}", a, b);
		}
	}
"#;
