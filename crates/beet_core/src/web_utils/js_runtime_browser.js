// @ts-nocheck
// deno-lint-ignore-file
//
// Browser version of the Beet Wasm Runtime
// Provides stub implementations for filesystem and environment operations
//
// This file is cached and will be replaced on hash change

globalThis.cwd = () => {
	return "/";
};

globalThis.exit = (code) => {
	console.log(`Process exit called with code: ${code}`);
};

globalThis.catch_no_abort_inner = (func) => {
	return func();
};

globalThis.read_file = (path) => {
	console.warn(`read_file not available in browser: ${path}`);
	return null;
};

globalThis.exists = (path) => {
	console.warn(`exists not available in browser: ${path}`);
	return false;
};

globalThis.create_dir_all = (path) => {
	console.warn(`create_dir_all not available in browser: ${path}`);
};

globalThis.write_file = (path, content) => {
	console.warn(`write_file not available in browser: ${path}`);
	return null;
};

globalThis.env_args = () => {
	return [];
};

globalThis.env_var = (key) => {
	return null;
};

globalThis.env_all = () => {
	return [];
};

// Test mode aliases
globalThis.test_cwd = globalThis.cwd;
globalThis.test_exit = globalThis.exit;
globalThis.test_exists = globalThis.exists;
globalThis.test_catch_no_abort_inner = globalThis.catch_no_abort_inner;
globalThis.test_read_file = globalThis.read_file;
globalThis.test_create_dir_all = globalThis.create_dir_all;
globalThis.test_write_file = globalThis.write_file;
globalThis.test_env_args = globalThis.env_args;
globalThis.test_env_var = globalThis.env_var;
globalThis.test_env_all = globalThis.env_all;
