// CIRCULAR: stringUtils → arrayUtils → objectUtils → stringUtils
import { unique } from './arrayUtils';

export function capitalize(str: string): string {
  return str.charAt(0).toUpperCase() + str.slice(1);
}

export function slugify(str: string): string {
  return str
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/(^-|-$)/g, '');
}

export function getUniqueWords(str: string): string[] {
  const words = str.split(/\s+/);
  return unique(words);
}
