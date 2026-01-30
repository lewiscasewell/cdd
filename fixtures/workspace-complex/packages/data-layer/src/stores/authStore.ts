// Auth store
import { userStore } from "./userStore";

export const authStore = {
  isLoggedIn: false,
  login() {
    this.isLoggedIn = true;
    userStore.loadUser();
  },
  logout() {
    this.isLoggedIn = false;
    userStore.clearUser();
  },
};
