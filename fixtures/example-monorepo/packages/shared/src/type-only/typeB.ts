// Type-only cycle: typeB -> typeA -> typeB (should be ignored with --ignore-type-imports)
import type { TypeA } from './typeA';

export interface TypeB {
  name: string;
  owner: TypeA;
}

export function createTypeB(name: string): TypeB {
  return { name, owner: null as any };
}
