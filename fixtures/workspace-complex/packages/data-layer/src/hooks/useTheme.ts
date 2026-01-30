// Theme hook - uses shared types
import type { Theme } from "@acme/shared/types";
import { themeStore } from "../stores/themeStore";

export function useTheme(): Theme {
  return themeStore.getTheme();
}
