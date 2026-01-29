// CIRCULAR: arrayUtils → objectUtils → stringUtils → arrayUtils
import { pick } from './objectUtils';

export function unique<T>(arr: T[]): T[] {
  return [...new Set(arr)];
}

export function chunk<T>(arr: T[], size: number): T[][] {
  const result: T[][] = [];
  for (let i = 0; i < arr.length; i += size) {
    result.push(arr.slice(i, i + size));
  }
  return result;
}

export function pluck<T extends Record<string, unknown>, K extends keyof T>(arr: T[], key: K): T[K][] {
  return arr.map(item => pick(item, [key])[key]);
}
