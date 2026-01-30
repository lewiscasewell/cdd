// Constants
export const COLORS = {
  primary: "#007bff",
  secondary: "#6c757d",
  success: "#28a745",
  danger: "#dc3545",
  warning: "#ffc107",
  info: "#17a2b8",
};

export const DEFAULT_THEME = {
  name: "default",
  colors: {
    primary: COLORS.primary,
    secondary: COLORS.secondary,
    background: "#ffffff",
    text: "#212529",
  },
  spacing: {
    small: 4,
    medium: 8,
    large: 16,
  },
};

export const API_ENDPOINTS = {
  auth: "/api/auth",
  users: "/api/users",
  themes: "/api/themes",
};
