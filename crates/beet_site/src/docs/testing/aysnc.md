---
title: Async
description: Running async tests
draft: true
sidebar:
  order: 3
---

## `#[tokio::test]` (native)

Sweet supports `#[tokio::test]` and any other macro that runs with the default test runner, and they will run in the same fashion.

## `#[wasm_bindgen_test` (wasm)

These tests are run in the `wasm_bindgen_test` runner and cannot be accessed by `sweet`. 

## `#[sweet::test]` (native,wasm)

This macro will run any test, be it native, wasm, sync or async.