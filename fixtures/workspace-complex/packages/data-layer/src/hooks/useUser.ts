// User hook - depends on auth hook (creates cycle)
import { useAuth } from "./useAuth";
import { userStore } from "../stores/userStore";

export function useUser() {
  const auth = useAuth();
  if (!auth.isAuthenticated) return null;
  return userStore.getCurrentUser();
}
