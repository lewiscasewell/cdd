// Helper utilities - no circular dependencies
export const helpers = {
  capitalize: (str: string) => str.charAt(0).toUpperCase() + str.slice(1),
  lowercase: (str: string) => str.toLowerCase(),
};
