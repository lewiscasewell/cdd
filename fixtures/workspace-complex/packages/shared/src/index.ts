// Shared package main entry
export * from "./types";
export * from "./constants";

// This creates a cross-package cycle:
// shared -> data-layer -> design-system (via stores) -> shared (via types/constants)
import { useModalStore } from "@acme/data-layer/stores/modalStore";

export function validateModalConfig(config: Record<string, unknown>) {
  const store = useModalStore();
  return Object.keys(config).length > 0;
}
