// Modal store - used by design-system modals
// Creates cross-package cycle: shared -> data-layer -> design-system -> shared
import { validateModalConfig } from "@acme/shared";

export function useModalStore() {
  return {
    isOpen: false,
    open() {
      validateModalConfig({});
      this.isOpen = true;
    },
    close() {
      this.isOpen = false;
    },
  };
}
