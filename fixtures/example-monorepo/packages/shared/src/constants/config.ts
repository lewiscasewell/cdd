// Non-circular: configuration constants
export const API_BASE_URL = 'https://api.example.com';
export const API_TIMEOUT = 30000;

export const PAGINATION = {
  DEFAULT_PAGE_SIZE: 20,
  MAX_PAGE_SIZE: 100,
} as const;

export const AUTH = {
  TOKEN_KEY: 'auth_token',
  REFRESH_KEY: 'refresh_token',
  EXPIRY_BUFFER: 60000, // 1 minute
} as const;
