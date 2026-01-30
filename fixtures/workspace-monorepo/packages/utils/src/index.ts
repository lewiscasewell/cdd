// Utils package - depends on core
import { getConfig } from "@test/core";

export function formatDate(date: Date): string {
  const config = getConfig();
  return date.toISOString();
}

export { helpers } from "./helpers";
