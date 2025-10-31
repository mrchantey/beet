# Refactoring Notes: StateBinder → BindContext

## Overview

This refactoring addresses several architectural issues to make the codebase more maintainable and easier to understand:

1. **Renamed `StateBinder` → `BindContext`** - Better reflects that this is a context for binding operations
2. **Eliminated awkward destructuring** - Pass `BindContext` directly instead of destructuring into `DirectiveContext`
3. **Consolidated scoping mechanism** - Built into `BindContext` via `.scoped()` method
4. **Removed code duplication** - `parseManifest` is now a static method on `BindContext`
5. **Centralized directive creation** - Static methods like `BindContext.handleEvent()` replace scattered helper functions
6. **Added test helper** - `BindContext.newTest()` reduces boilerplate in tests

## Migration Guide

### Basic Usage

**Before:**
```typescript
import { StateBinder } from "./StateBinder";

const stateBinder = new StateBinder(repo);
await stateBinder.init();
```

**After:**
```typescript
import { BindContext } from "./BindContext";

const bindContext = new BindContext(repo);
await bindContext.init();
```

### Directive Creation

**Before:**
```typescript
import { createHandleEvent, createRenderText, createRenderList } from "./directives";

const manifest = {
  state_directives: [
    createHandleEvent({ el_state_id: 0, field_path: "count", event: "click", action: "increment" }),
    createRenderText({ el_state_id: 1, field_path: "count", template: "%VALUE%" }),
    createRenderList({ el_state_id: 2, field_path: "items", template_id: 3 })
  ]
};
```

**After:**
```typescript
import { BindContext } from "./BindContext";

const manifest = {
  state_directives: [
    BindContext.handleEvent({ el_state_id: 0, field_path: "count", event: "click", action: "increment" }),
    BindContext.renderText({ el_state_id: 1, field_path: "count", template: "%VALUE%" }),
    BindContext.renderList({ el_state_id: 2, field_path: "items", template_id: 3 })
  ]
};
```

### Test Setup

**Before:**
```typescript
let stateBinder: StateBinder;

beforeEach(async () => {
  document.body.innerHTML = "";
  localStorage.clear();
  stateBinder = new StateBinder(new Repo());
});

afterEach(() => {
  stateBinder.destroy();
});
```

**After:**
```typescript
let bindContext: BindContext;

beforeEach(async () => {
  bindContext = BindContext.newTest();
});

afterEach(() => {
  bindContext.destroy();
});
```

## Architecture Improvements

### 1. DirectiveContext Removed

The `DirectiveContext` type has been removed. Directive binding functions now receive the `BindContext` directly.

**Before:**
```typescript
type DirectiveContext = {
  docHandle: DocHandle<any>;
  getValueByPath: (doc: any, path?: string) => any;
  setValueByPath: (doc: any, path: string | undefined, value: any) => void;
  findElementForDirective: (root: Element, directive: StateDirective) => Result<Element, string>;
};

function bindHandleEvent(element: Element, config: HandleEvent, context: DirectiveContext) {
  // ...
}
```

**After:**
```typescript
function bindHandleEvent(element: Element, config: HandleEvent, context: BindContext) {
  context.docHandle!.change((doc) => {
    context.setValueByPath(doc, config.field_path, newValue);
  });
}
```

### 2. Scoping Mechanism

The scoping mechanism for list items is now integrated into `BindContext` via the `.scoped()` method.

**Before:**
```typescript
// Separate scoping logic with path manipulation
function scopeDirectiveToItem(directive: StateDirective, itemPath: string): StateDirective {
  const fieldPath = directive.field_path;
  const scopedPath = fieldPath ? `${itemPath}.${fieldPath}` : itemPath;
  return { ...directive, field_path: scopedPath };
}

function bindDirectiveScoped(element: Element, directive: StateDirective, context: DirectiveContext, disposers: Array<() => void>) {
  // Custom binding logic for scoped context
}
```

**After:**
```typescript
// Clean scoping via context method
const itemPath = `${arrayPath}[${index}]`;
const scopedContext = context.scoped(itemPath);

// Use the scoped context normally
scopedContext.bindDirective(element, directive);
```

The scoped context automatically prefixes all field paths, making the scoping transparent to directive binding logic.

### 3. Consolidated Parsing

Manifest parsing is now a static method on `BindContext`, eliminating duplication.

**Before:**
```typescript
// In StateBinder.ts
private parseManifest(script: HTMLScriptElement): Result<StateManifest, string> { /* ... */ }

// In RenderList.ts (duplicate)
function parseManifest(script: HTMLScriptElement): Result<StateManifest, string> { /* ... */ }
```

**After:**
```typescript
// Single implementation as static method
BindContext.parseManifest(script);
```

### 4. Centralized Directive Creation

All directive creation helpers are now static methods on `BindContext`, making them easier to discover.

**Benefits:**
- Single import point (`BindContext`)
- Autocomplete-friendly (type `BindContext.` to see all available directives)
- Clear ownership (directives belong to the binding context)

## Files Changed

- `src/StateBinder.ts` → `src/BindContext.ts` (renamed and refactored)
- `src/directives/types.ts` - Removed `DirectiveContext` and `PartialBy` types
- `src/directives/HandleEvent.ts` - Removed `createHandleEvent`, updated to use `BindContext`
- `src/directives/RenderText.ts` - Removed `createRenderText` and `bindRenderTextScoped`, updated to use `BindContext`
- `src/directives/RenderList.ts` - Removed `createRenderList`, `parseManifest`, `scopeDirectiveToItem`, and `bindDirectiveScoped`, uses scoped contexts instead
- `src/directives/index.ts` - Updated exports
- All test files - Updated to use `BindContext.newTest()` and static directive methods
- Demo files - Updated imports and method calls

## Breaking Changes

1. `StateBinder` class renamed to `BindContext`
2. Helper functions `createHandleEvent`, `createRenderText`, `createRenderList` removed in favor of `BindContext.handleEvent()`, `BindContext.renderText()`, `BindContext.renderList()`
3. `DirectiveContext` type removed
4. `bindRenderTextScoped` removed (scoping now handled by `BindContext.scoped()`)

## Non-Breaking Changes

- All tests still pass
- API surface is smaller and more cohesive
- No changes to manifest format or HTML structure
- Directive binding behavior is unchanged