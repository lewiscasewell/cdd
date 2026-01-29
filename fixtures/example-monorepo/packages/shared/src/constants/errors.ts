// Non-circular: error constants
export const ERROR_CODES = {
  UNAUTHORIZED: 'UNAUTHORIZED',
  NOT_FOUND: 'NOT_FOUND',
  VALIDATION_ERROR: 'VALIDATION_ERROR',
  SERVER_ERROR: 'SERVER_ERROR',
} as const;

export const ERROR_MESSAGES = {
  [ERROR_CODES.UNAUTHORIZED]: 'You are not authorized to perform this action',
  [ERROR_CODES.NOT_FOUND]: 'The requested resource was not found',
  [ERROR_CODES.VALIDATION_ERROR]: 'The provided data is invalid',
  [ERROR_CODES.SERVER_ERROR]: 'An unexpected error occurred',
} as const;
