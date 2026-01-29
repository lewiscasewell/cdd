# Example Monorepo - CDD Test Fixture

This is a **buildable** test fixture for the Circular Dependency Detector (CDD) tool. It contains realistic TypeScript code patterns with both circular and non-circular dependencies.

## Setup

```bash
cd fixtures/example-monorepo
npm install
npm run build
```

## Structure

```
packages/
├── api/          # Backend API services
├── web/          # Frontend React application
└── shared/       # Shared utilities and types
```

## Expected Circular Dependencies

### 1. API Services (2-way cycle)
- `userService.ts` ↔ `orderService.ts`
- Cause: Services need to reference each other for fetching related data

### 2. Web Components (3-way cycle)
- `Button.tsx` → `Modal.tsx` → `Form.tsx` → `Button.tsx`
- Cause: UI components that compose each other

### 3. Web Hooks (2-way cycle)
- `useAuth.ts` ↔ `useUser.ts`
- Cause: Auth state depends on user, user fetching depends on auth

### 4. Shared Utils (3-way cycle)
- `stringUtils.ts` → `arrayUtils.ts` → `objectUtils.ts` → `stringUtils.ts`
- Cause: Utility functions that build upon each other

### 5. Type-Only Cycle (2-way)
- `type-only/typeA.ts` ↔ `type-only/typeB.ts`
- Cause: Types that reference each other
- **Note:** This cycle can be ignored with `--ignore-type-imports` since type imports are erased at compile time

## Non-Circular Patterns

- **Linear**: `userController.ts` → `userService.ts` → `user.ts`
- **Tree**: `orderController.ts` → both `orderService.ts` and `userService.ts`
- **Diamond**: Multiple files importing from `types/index.ts`
- **Standalone**: `Card.tsx`, `useLocalStorage.ts`, `dateUtils.ts`

## Running CDD

```bash
# From the cdd project root
cargo run -- ./fixtures/example-monorepo/packages

# Or with the installed binary
cdd ./fixtures/example-monorepo/packages
```

## Expected Output

```
✖ Found 5 circular dependencies!

1) api/src/services/orderService.ts > api/src/services/userService.ts
2) shared/src/type-only/typeA.ts > shared/src/type-only/typeB.ts
3) shared/src/utils/arrayUtils.ts > shared/src/utils/objectUtils.ts > shared/src/utils/stringUtils.ts
4) web/src/hooks/useAuth.ts > web/src/hooks/useUser.ts
5) web/src/components/Button.tsx > web/src/components/Modal.tsx > web/src/components/Form.tsx
```

With `--ignore-type-imports`:
```
✖ Found 4 circular dependencies!
# (type-only/typeA.ts ↔ typeB.ts cycle is excluded)
```

## Scanning Source vs Built Output

| Scan Target | Cycles | Notes |
|-------------|--------|-------|
| Source (`.ts/.tsx`) | 5 | All imports detected |
| Source + `--ignore-type-imports` | 4 | Type-only cycle excluded |
| Built (`dist/*.js`) | 3 | TypeScript optimizes away type-only imports |

### Why does built output have fewer cycles?

1. **Type-only imports are erased** - `import type { Foo }` doesn't exist in compiled JS
2. **Dependency injection types are erased** - The API services cycle (`userService ↔ orderService`) disappears because the imports were only used as constructor parameter types, not runtime values

This demonstrates why scanning **both** source and built output can be valuable:
- **Source scanning**: Catches all architectural issues
- **Built scanning**: Shows actual runtime circular dependencies

## Commands

```bash
# Scan source files only (excludes dist)
npm run check-cycles:src

# Scan built output only (excludes src)
npm run check-cycles:dist

# Scan built output, ignoring type imports
npm run check-cycles:runtime
```
