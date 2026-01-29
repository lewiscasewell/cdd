// CIRCULAR: useAuth → useUser → useAuth
import { useState, useEffect } from 'react';
import { useUser, User } from './useUser';

interface AuthState {
  isAuthenticated: boolean;
  user: User | null;
  login: (email: string, password: string) => Promise<void>;
  logout: () => void;
}

export function useAuth(): AuthState {
  const [isAuthenticated, setIsAuthenticated] = useState<boolean>(false);
  const { user, refetchUser } = useUser();

  useEffect(() => {
    setIsAuthenticated(!!user);
  }, [user]);

  const login = async (email: string, password: string): Promise<void> => {
    // Simulated login
    setIsAuthenticated(true);
    refetchUser();
  };

  const logout = (): void => {
    setIsAuthenticated(false);
  };

  return { isAuthenticated, user, login, logout };
}
