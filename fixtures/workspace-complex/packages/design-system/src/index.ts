// Main entry point - re-exports all components
export * from "./components/buttons";
export * from "./components/forms";
export * from "./components/modals";

// Uses shared types
import type { Theme } from "@acme/shared/types";

export interface DesignSystemConfig {
  theme: Theme;
}
