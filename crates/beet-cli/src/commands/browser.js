// The browser host for a beet wasm module: the browser twin of `deno.ts`.
//
// Installs the `js_runtime` globals a wasm module may import, then boots the
// module through the same contract every host uses: `init()`, then await the
// exported `test_start` (a test suite) or `start` (an app binary), recording
// the exit code on `globalThis.__beet_exit` for a supervisor (eg the
// `run-wasm` webdriver driver) to poll. The `exit` global remains the
// error-path escape: a module that aborts mid-run reports through it before
// the awaited promise ever settles.
//
// Each global is also installed under the `test_` alias: a `--lib` test
// build links beet_core twice, so the bindgen glue imports both names.

import init, * as bindgen from "./bindgen.js";

const exit = (code) => {
	globalThis.__beet_exit = code;
};
const passthrough = (func) => func();
// runner-provided env (eg WORKSPACE_ROOT), absent when served statically
const env = await fetch("./env.json")
	.then((res) => (res.ok ? res.json() : {}))
	.catch(() => ({}));
const env_var = (key) => env[key] ?? null;
Object.assign(globalThis, {
	exit,
	test_exit: exit,
	catch_no_abort_inner: passthrough,
	test_catch_no_abort_inner: passthrough,
	env_var,
	test_env_var: env_var,
});

try {
	await init();
	const entry = bindgen.test_start ?? bindgen.start;
	if (entry) {
		exit((await entry()) ?? 0);
	}
	// a module with no start export is a daemon: it drives its own tasks and
	// reports through the exit global when it terminates
} catch (err) {
	console.error("wasm boot failed:", err);
	exit(101);
}
