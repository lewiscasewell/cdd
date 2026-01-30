// Theme store
import type { Theme } from "@acme/shared/types";
import { DEFAULT_THEME } from "@acme/shared/constants";

export const themeStore = {
  theme: DEFAULT_THEME as Theme,
  getTheme(): Theme {
    return this.theme;
  },
  setTheme(theme: Theme) {
    this.theme = theme;
  },
};
