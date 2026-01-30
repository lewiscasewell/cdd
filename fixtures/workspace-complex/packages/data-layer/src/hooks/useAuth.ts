// Auth hook - depends on user hook (creates cycle)
import { useUser } from "./useUser";
import { authStore } from "../stores/authStore";

export function useAuth() {
  const user = useUser();
  return {
    isAuthenticated: !!user,
    login: () => authStore.login(),
    logout: () => authStore.logout(),
  };
}
