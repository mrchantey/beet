use crate::prelude::*;
use anyhow::Result;
#[cfg(feature = "e2e")]
use beet_utils::prelude::*;
use std::process::Child;


/// start the chromedriver and return the child process
pub fn run_chromedriver(config: &TestRunnerConfig) -> Result<Option<Child>> {
	if !config.e2e {
		return Ok(None);
	}
	#[cfg(not(feature = "e2e"))]
	{
		anyhow::bail!(
			"e2e feature must be enabled for use with the e2e flag, try 'cargo test --features=sweet/e2e -- --e2e'"
		)
	}
	#[cfg(feature = "e2e")]
	{
		std::process::Command::new("nix-shell")
			.args(&[
				"-p",
				"chromium",
				"chromedriver",
				"--run",
				&format!(
					"chromedriver --port={DEFAULT_WEBDRIVER_PORT} --silent"
				),
			])
			.spawn()?
			.xsome()
			.xok()
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
