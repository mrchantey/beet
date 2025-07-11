

export function add(a: number, b: number): number {
  return a + b;
}

// Tests
import { assertEquals } from "@std/assert";

Deno.test(function addTest() {
  assertEquals(add(2, 3), 5);
});

// Learn more at https://docs.deno.com/runtime/manual/examples/module_metadata#concepts
if (import.meta.main) {
  console.log("Add 2 + 3 =", add(2, 3));
}
