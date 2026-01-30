// User store - depends on auth store (creates cycle)
import { authStore } from "./authStore";
import type { User } from "@acme/shared/types";

let currentUser: User | null = null;

export const userStore = {
  getCurrentUser(): User | null {
    if (!authStore.isLoggedIn) return null;
    return currentUser;
  },
  loadUser() {
    currentUser = { id: "1", name: "Test User" };
  },
  clearUser() {
    currentUser = null;
  },
};
