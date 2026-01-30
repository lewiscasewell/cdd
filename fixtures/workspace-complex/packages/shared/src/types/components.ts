// Component types
export type ButtonVariant = "primary" | "secondary" | "danger" | "submit" | "confirm" | "cancel";

export interface ModalConfig {
  title: string;
  closable?: boolean;
  size?: "small" | "medium" | "large";
}
