// Non-circular: validation utilities
export function isValidEmail(email: string): boolean {
  return /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email);
}

export function isValidPhone(phone: string): boolean {
  return /^\d{10}$/.test(phone.replace(/\D/g, ''));
}

export function isRequired(value: string): boolean {
  return value.trim().length > 0;
}
