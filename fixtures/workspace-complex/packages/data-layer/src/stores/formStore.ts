// Form store - used by design-system forms
export function useFormStore() {
  return {
    values: {} as Record<string, string>,
    setValue(name: string, value: string) {
      this.values[name] = value;
    },
    getValue(name: string) {
      return this.values[name];
    },
  };
}
