// Button components
import { COLORS } from "@acme/shared/constants";
import type { ButtonVariant } from "@acme/shared/types";

export function Button({ variant }: { variant: ButtonVariant }) {
  return `<button style="color: ${COLORS.primary}">${variant}</button>`;
}

export function IconButton({ icon }: { icon: string }) {
  return `<button>${icon}</button>`;
}

// Deep import - uses modal for confirmation buttons
import { ConfirmModal } from "../modals";

export function ConfirmButton({ onConfirm }: { onConfirm: () => void }) {
  return ConfirmModal({ onConfirm, children: Button({ variant: "primary" }) });
}
