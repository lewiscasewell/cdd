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
# Scan a directory
cdd ./src

# Exclude directories
cdd --exclude node_modules --exclude dist ./src

# Ignore type-only imports (recommended for TypeScript)
cdd --ignore-type-imports ./src

# Scan built JavaScript output
cdd --exclude src ./dist

# CI mode: fail if cycles don't match expected count
cdd -n 0 ./src  # Fail if any cycles found
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
✖ Found 2 circular dependencies!

1) src/services/userService.ts > src/services/orderService.ts
2) src/components/Button.tsx > src/components/Modal.tsx > src/components/Form.tsx
```

This means:
- `userService.ts` imports `orderService.ts`, which imports `userService.ts`
- `Button.tsx` → `Modal.tsx` → `Form.tsx` → `Button.tsx`

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
