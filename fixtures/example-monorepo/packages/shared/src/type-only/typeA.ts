// Type-only cycle: typeA -> typeB -> typeA (should be ignored with --ignore-type-imports)
import type { TypeB } from './typeB';

export interface TypeA {
  id: string;
  related: TypeB;
}

export function createTypeA(id: string): TypeA {
  return { id, related: null as any };
}
