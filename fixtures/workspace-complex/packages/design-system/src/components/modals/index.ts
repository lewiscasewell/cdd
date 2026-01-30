// Modal components
import { Button } from "../buttons";
import { useModalStore } from "@acme/data-layer/stores/modalStore";

export function Modal({ children, title }: { children: string; title: string }) {
  const store = useModalStore();
  return `<div class="modal"><h2>${title}</h2>${children}</div>`;
}

export function ConfirmModal({ onConfirm, children }: { onConfirm: () => void; children: string }) {
  return Modal({
    title: "Confirm",
    children: `${children}${Button({ variant: "confirm" })}${Button({ variant: "cancel" })}`,
  });
}

// Creates cycle: modals -> forms -> buttons -> modals
import { Form } from "../forms";

export function FormModal({ title }: { title: string }) {
  return Modal({ title, children: Form({ children: "" }) });
}
