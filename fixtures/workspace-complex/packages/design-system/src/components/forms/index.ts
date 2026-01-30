// Form components
import { Button } from "../buttons";
import { useFormStore } from "@acme/data-layer/stores/formStore";

export function Form({ children }: { children: string }) {
  const store = useFormStore();
  return `<form>${children}${Button({ variant: "submit" })}</form>`;
}

export function Input({ name }: { name: string }) {
  return `<input name="${name}" />`;
}

export function FormField({ label, name }: { label: string; name: string }) {
  return `<div><label>${label}</label>${Input({ name })}</div>`;
}
