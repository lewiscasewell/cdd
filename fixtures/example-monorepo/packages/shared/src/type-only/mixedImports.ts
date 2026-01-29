// Mixed import: type + value from same source
// This should NOT be ignored because it has a value import
import { type TypeA, createTypeA } from './typeA';

export function example(): TypeA {
  return createTypeA('test');
}
