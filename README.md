# Circular dependency detector (CDD)
## Detect circular dependencies in JS projects

inspired by [madge](https://github.com/pahen/madge) (a JS implementation), but wanted to make it faster and output more comprehensive cycles.

## Usage

```bash
cdd [OPTIONS] [DIR]
```

## Example

```bash
cdd -- --exclude node_modules ./src
```

## Supported files

`.ts`, `.tsx`, `.js`, `.jsx`, `.cjs`, `.mjs`

## How it works

1. Parse all files in the directory and extract all imports
2. Create a graph of the imports
3. Find all cycles in the graph
4. Output the cycles

If a cycle is detected will return a non-zero exit code. If no cycles are detected will return a zero exit code.

An example output of the cycle could be:
```
✖ Found 1 circular dependencies!

1) packages/api/src/a.ts > packages/api/src/c/index.ts > packages/api/src/c/a.ts > packages/api/src/c/b.ts > packages/api/src/b.ts
```

This can be interpreted as:
- `packages/api/src/a.ts` imports `packages/api/src/c/index.ts`
- `packages/api/src/c/index.ts` imports `packages/api/src/c/a.ts`
- `packages/api/src/c/a.ts` imports `packages/api/src/c/b.ts`
- `packages/api/src/c/b.ts` imports `packages/api/src/b.ts`
- `packages/api/src/b.ts` imports `packages/api/src/a.ts`

## steps to resolve this cycle

1. Identify Shared Responsibilities
Determine if any modules share common functionalities that can be abstracted into separate modules. This often helps in reducing direct dependencies.
2. Refactor to Remove Direct Dependencies
Here's how you can approach refactoring based on your detected cycle:
Extract Common Functionality:

a. If both `a.ts` and `b.ts` share some logic, extract it into a new module (e.g., `common.ts`).
Decouple Modules Using Interfaces:

b. Instead of directly importing modules, use interfaces or abstractions to define dependencies.
Introduce an Intermediary Module:

c. Create a new module that coordinates interactions between `a.ts` and `b.ts`, thereby eliminating direct circular imports.

3. Test and Validate
After refactoring, test your changes to ensure that the circular dependency has been resolved. Run your tests and check for any regressions.
