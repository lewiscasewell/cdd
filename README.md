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

## Why i chose single comprehensive cycle output over multiple smaller cycles
unlike madge, i chose to use a long chain of dependencies to represent the cycle instead of multiple smaller cycles because

- Single comprehensive cycle output is easier to understand and debug.
- It provides a clear picture of the dependencies that need to be resolved.

Example:

Multiple Smaller Cycles:
`a.ts > b.ts > a.ts`
`a.ts > c/index.ts > c/b.ts > c/a.ts > a.ts`
`a.ts > c/index.ts > c/a.ts > a.ts`

- Single Comprehensive Cycle (SCC):
`a.ts > c/index.ts > c/a.ts > c/b.ts > b.ts > a.ts`

## Steps to Cross-Compile for Linux on macOS

Install Homebrew Tools for Cross-Compilation Use Homebrew to install a Linux-compatible gcc toolchain.
```bash
brew tap messense/macos-cross-toolchains
brew install x86_64-unknown-linux-gnu
```
This installs the x86_64-unknown-linux-gnu-gcc cross-compiler, which is required.

Add the Toolchain to the Path Update your environment to include the cross-compiler.
```bash
export CC_x86_64_unknown_linux_gnu=x86_64-unknown-linux-gnu-gcc
export CXX_x86_64_unknown_linux_gnu=x86_64-unknown-linux-gnu-g++
```
Add these lines to your shell config (e.g., .zshrc or .bashrc) if you plan to use this often.

Install Rust Target for Linux Use rustup to install the x86_64-unknown-linux-gnu target.
```bash
rustup target add x86_64-unknown-linux-gnu
```

Build the Project for the Target Use the --target flag to cross-compile the binary.
```bash
cargo build --release --target x86_64-unknown-linux-gnu
```