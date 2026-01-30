// Core package - depends on UI (creates circular dependency!)
import { UI_VERSION } from "@test/ui";

export function getConfig() {
  return {
    version: UI_VERSION,
    name: "core",
  };
}
