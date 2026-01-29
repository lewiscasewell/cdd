// CIRCULAR: useUser → useAuth → useUser
import { useState, useCallback } from 'react';
import { useAuth } from './useAuth';

export interface User {
  id: string;
  name: string;
  email: string;
}

interface UserState {
  user: User | null;
  refetchUser: () => void;
}

export function useUser(): UserState {
  const [user, setUser] = useState<User | null>(null);
  // Note: This creates a circular dependency that TypeScript can handle
  // because we use explicit types, but it's still a code smell
  const { isAuthenticated } = useAuth();

  const refetchUser = useCallback((): void => {
    if (isAuthenticated) {
      // Simulated fetch
      setUser({ id: '1', name: 'John', email: 'john@example.com' });
    }
  }, [isAuthenticated]);

  return { user, refetchUser };
}
