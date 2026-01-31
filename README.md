# Circular Dependency Detector (CDD)

Fast circular dependency detection for JavaScript and TypeScript projects.

Inspired by [madge](https://github.com/pahen/madge), but built in Rust for speed and comprehensive cycle output.

## Installation

Download the latest release for your platform from [Releases](https://github.com/lewiscasewell/cdd/releases).

Or build from source:
```bash
cargo install --path .
```

## Usage

```bash
cdd [OPTIONS] <DIR>
```

### Examples

```bash
# Scan a directory (auto-detects workspace and tsconfig)
cdd ./src

# Exclude directories
cdd --exclude node_modules --exclude dist ./src

# Ignore type-only imports (recommended for TypeScript)
cdd --ignore-type-imports ./src

# CI mode: fail if any new cycles are found
cdd -n 0 ./src

# Initialize config with current cycles as baseline
cdd --init ./src

# Watch mode: re-run on file changes
cdd --watch ./src

# JSON output for tooling integration
cdd --json ./src

# CI with hash validation (detects when cycles change)
cdd --expected-hash abc123def456 ./src
```

## Options

```
Arguments:
  <DIR>  The root directory to analyze

Options:
  -e, --exclude <EXCLUDE>        Directories to exclude (can be used multiple times)
  -t, --ignore-type-imports      Ignore type-only imports (import type { Foo })
  -d, --debug                    Enable debug logging
  -n, --numberOfCycles <N>       Expected number of cycles [default: 0]
  -s, --silent                   Suppress all output
  -w, --watch                    Watch mode: re-run analysis on file changes
      --tsconfig <PATH>          Path to tsconfig.json (auto-detected by default)
      --no-tsconfig              Disable tsconfig auto-detection
      --no-workspace             Disable workspace auto-detection
      --json                     Output results as JSON
      --expected-hash <HASH>     Fail if cycle hash doesn't match (for CI)
      --allowlist <PATH>         Path to file listing allowed cycles
      --update-hash              Update expected_hash in config file
      --init                     Initialize config with current cycles as baseline
  -h, --help                     Print help
  -V, --version                  Print version
```

## Supported Files

| Extension | Syntax |
|-----------|--------|
| `.ts`     | TypeScript |
| `.tsx`    | TypeScript + JSX |
| `.js`, `.mjs` | ES Modules |
| `.jsx`    | ES Modules + JSX |
| `.cjs`    | CommonJS |

## Supported Import Types

- ES Module imports: `import { foo } from './foo'`
- Type-only imports: `import type { Foo } from './foo'` (skipped with `-t`)
- Dynamic imports: `const mod = await import('./foo')`
- CommonJS: `const foo = require('./foo')`
- Re-exports: `export * from './foo'`

## Example Output

```
X Found 2 circular dependencies!

1) Circular dependency [b19d1af3c370]:
   src/services/orderService.ts:3
   | import { UserService } from './userService';
   v
   src/services/userService.ts:3
   | import { OrderService } from './orderService';
   ^-- (cycle)

2) Circular dependency [44ff849f72f4]:
   src/components/Button.tsx:3
   | import { Modal } from './Modal';
   v
   src/components/Modal.tsx:3
   | import { Form } from './Form';
   v
   src/components/Form.tsx:3
   | import { Button } from './Button';
   ^-- (cycle)
```

Each cycle shows:
- A unique hash for identification
- The exact file and line number of each import
- The import statement causing the dependency

## Configuration File

CDD supports configuration files to avoid repeating options. Create `.cddrc.json` or `cdd.config.json` in your project root:

```json
{
  "exclude": ["node_modules", "dist", "__tests__"],
  "ignore_type_imports": true,
  "expected_cycles": 0,
  "expected_hash": "abc123def456",
  "allowed_cycles": [
    {
      "files": ["src/a.ts", "src/b.ts"],
      "reason": "Known issue, tracked in JIRA-123"
    }
  ]
}
```

### Quick Setup with `--init`

The easiest way to set up a config is to use `--init`:

```bash
cdd --init ./src
```

This creates `.cddrc.json` with all current cycles in the allowlist. After init:
- Existing cycles are allowed (won't cause CI failures)
- New cycles will cause failures
- You can gradually fix cycles and remove them from the allowlist

CDD searches for config files starting from the target directory and walking up. CLI arguments take precedence over config file values.

## CI Integration

### Basic: Fail on Any Cycles

```bash
cdd -n 0 ./src
```

### Recommended: Hash-Based Validation

Use `--expected-hash` to detect when cycles change, even if the count stays the same:

```bash
# First, get the current hash
cdd ./src
# Output: Cycles hash: abc123def456

# Then use it in CI
cdd --expected-hash abc123def456 ./src
```

Update the hash when cycles change intentionally:
```bash
cdd --update-hash ./src
```

### JSON Output

Use `--json` for integration with other tools:

```bash
cdd --json ./src
```

```json
{
  "total_files": 150,
  "total_cycles": 2,
  "cycles_hash": "abc123def456",
  "cycles": [
    {
      "hash": "b19d1af3c370",
      "edges": [
        {
          "from_file": "src/a.ts",
          "to_file": "src/b.ts",
          "line": 3,
          "import_text": "import { b } from './b';"
        }
      ]
    }
  ]
}
```

## Watch Mode

> **Note:** Watch mode is only available when building from source with the `watch` feature (enabled by default). Pre-built release binaries do not include watch mode to ensure cross-platform compatibility.

Use `--watch` to continuously monitor for changes and re-run analysis:

```bash
cdd --watch ./src
```

The terminal clears between runs, showing fresh results each time. Press `Ctrl+C` to stop.

## TypeScript Path Aliases

CDD automatically detects and uses `tsconfig.json` in your project root for path alias resolution. You can also specify a custom path:

```bash
cdd --tsconfig ./packages/app/tsconfig.json ./src
```

Use `--no-tsconfig` to disable auto-detection.

Supports:
- `compilerOptions.paths` mappings (e.g., `@/*` → `src/*`)
- `compilerOptions.baseUrl` for non-relative imports
- `extends` chains (inherits from parent configs)

Example `tsconfig.json`:
```json
{
  "compilerOptions": {
    "baseUrl": ".",
    "paths": {
      "@/*": ["src/*"],
      "@components/*": ["src/components/*"]
    }
  }
}
```

## Monorepo Workspace Resolution

CDD automatically detects monorepo workspaces and resolves bare package imports like `@acme/ui` to their actual source files:

```bash
cdd ./packages
```

This enables detection of cross-package cycles:

```
1) Circular dependency [d1bd5c39d882]:
   packages/core/src/index.ts:2
   | import { UI_VERSION } from "@acme/ui";
   v
   packages/ui/src/index.ts:2
   | import { formatDate } from "@acme/utils";
   v
   packages/utils/src/index.ts:2
   | import { getConfig } from "@acme/core";
   ^-- (cycle)
```

Use `--no-workspace` to disable auto-detection if needed.

### Supported Workspace Formats

| Format | Config File | Field |
|--------|-------------|-------|
| npm/yarn | `package.json` | `workspaces` |
| pnpm | `pnpm-workspace.yaml` | `packages` |

### Package Resolution

CDD resolves package imports in this order:

1. **`exports` field** - Conditional exports (`import`/`require`/`default`), subpath exports, wildcards
2. **`module` field** - ES module entry point
3. **`main` field** - CommonJS entry point
4. **Convention** - `src/index.ts`, `index.ts`, `index.js`

### Subpath Imports

Deep imports into packages are resolved via the `exports` field:

```typescript
// Resolves @acme/ui/button to packages/ui/src/components/button.ts
import { Button } from "@acme/ui/button";
```

With package.json:
```json
{
  "name": "@acme/ui",
  "exports": {
    ".": "./src/index.ts",
    "./button": "./src/components/button.ts",
    "./*": "./src/*.ts"
  }
}
```

## Type-Only Imports

TypeScript's `import type` statements are erased at compile time and don't cause runtime circular dependencies. Use `--ignore-type-imports` to skip these:

```bash
# Source has 5 cycles, but only 4 are runtime cycles
cdd ./src                        # Reports 5 cycles
cdd --ignore-type-imports ./src  # Reports 4 cycles
```

## Scanning Built Output

You can scan compiled JavaScript to see actual runtime dependencies:

```bash
# Build your project first
npm run build

# Scan source vs built output
cdd --exclude dist ./src    # TypeScript source (includes type imports)
cdd --exclude src ./dist    # Built JavaScript (type imports erased)
```

Built output often has fewer cycles because TypeScript erases:
- `import type` statements
- Imports used only for type annotations

## How It Works

1. Recursively find all JS/TS files in the directory
2. Parse each file and extract imports using [SWC](https://swc.rs/)
3. Build a dependency graph
4. Find strongly connected components using Kosaraju's algorithm
5. Report unique cycles

Exit codes:
- `0` - Success (cycles match expected count, or no cycles if `-n` not specified)
- `1` - Failure (cycles found, or count doesn't match `-n`)

## Why Single Comprehensive Cycles?

Unlike madge, CDD outputs one comprehensive chain per cycle instead of multiple overlapping fragments:

**Multiple smaller cycles (madge style):**
```
a.ts > b.ts > a.ts
a.ts > c/index.ts > c/b.ts > a.ts
```

**Single comprehensive cycle (CDD):**
```
a.ts > b.ts > c/index.ts > c/b.ts
```

This is easier to understand and debug.

## Resolving Circular Dependencies

1. **Extract shared code** - Move common functionality to a separate module
2. **Use interfaces** - Depend on abstractions instead of concrete implementations
3. **Introduce a coordinator** - Create a module that coordinates interactions
4. **Lazy loading** - Use dynamic imports for non-critical dependencies

## Development

```bash
# Run tests
cargo test

# Build release
cargo build --release

# Run against test fixture
./target/release/cdd ./fixtures/example-monorepo/packages --exclude dist
```

### Release Scripts

```bash
# Bump version
./scripts/bump-version.sh 0.4.0

# Create and push release (runs tests, creates tag, pushes)
./scripts/release.sh
```

### Cross-Compile for Linux on macOS

Install Homebrew tools for cross-compilation:
```bash
brew tap messense/macos-cross-toolchains
brew install x86_64-unknown-linux-gnu
```

Add the toolchain to your environment:
```bash
export CC_x86_64_unknown_linux_gnu=x86_64-unknown-linux-gnu-gcc
export CXX_x86_64_unknown_linux_gnu=x86_64-unknown-linux-gnu-g++
```

Install the Rust target and build:
```bash
rustup target add x86_64-unknown-linux-gnu
cargo build --release --target x86_64-unknown-linux-gnu
```

## License

MIT
