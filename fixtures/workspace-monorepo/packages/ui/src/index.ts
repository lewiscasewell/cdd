// UI package - depends on utils
import { formatDate } from "@test/utils";

export function renderDate(date: Date): string {
  return `<span>${formatDate(date)}</span>`;
}

export const UI_VERSION = "1.0.0";
