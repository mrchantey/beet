use crate::prelude::TestRunnerConfig;
use anyhow::Result;
use std::env;
use std::path::PathBuf;
use std::process::Child;
use std::process::Command;
use sweet_utils::utils::PipelineTarget;


/// if the e2e flag is set, start the chromedriver
/// and return the child process
///
/// TODO install if doesnt exist
pub fn try_run_e2e(config: &TestRunnerConfig) -> Result<Option<Child>> {
	if config.e2e {
		let home_dir = env::var("HOME")
			.map_err(|e| anyhow::anyhow!("Failed to get HOME: {e}"))?;
		let chromedriver_path = PathBuf::from(home_dir)
			.join("chrome-for-testing/chromedriver-linux64/chromedriver");
		Command::new(chromedriver_path)
			.arg("--port=4444")
			.arg("--silent")
			// .arg("--verbose")
			.spawn()
			.map_err(|e| anyhow::anyhow!("Failed to start chromedriver: {e}"))?
			.xsome()
			.xok()
	} else {
		Ok(None)
	}
}


/*
Options
--port=PORT                     port to listen on
--adb-port=PORT                 adb server port
--log-path=FILE                 write server log to file instead of stderr, increases log level to INFO
--log-level=LEVEL               set log level: ALL, DEBUG, INFO, WARNING, SEVERE, OFF
--verbose                       log verbosely (equivalent to --log-level=ALL)
--silent                        log nothing (equivalent to --log-level=OFF)
--append-log                    append log file instead of rewriting
--replayable                    (experimental) log verbosely and don't truncate long strings so that the log can be replayed.
--version                       print the version number and exit
--url-base                      base URL path prefix for commands, e.g. wd/url
--readable-timestamp            add readable timestamps to log
--enable-chrome-logs            show logs from the browser (overrides other logging options)
--bidi-mapper-path              custom bidi mapper path
--disable-dev-shm-usage         do not use /dev/shm (add this switch if seeing errors related to shared memory)
--ignore-explicit-port          (experimental) ignore the port specified explicitly, find a free port instead
--allowed-ips=LIST              comma-separated allowlist of remote IP addresses which are allowed to connect to ChromeDriver
--allowed-origins=LIST          comma-separated allowlist of request origins which are allowed to connect to ChromeDriver. Using `*` to allow any host origin is dangerous!
*/
