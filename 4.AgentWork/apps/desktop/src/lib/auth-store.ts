// 全局登录状态管理
// 简单事件机制：未登录时点击受限功能触发登录弹窗

import { create } from "zustand";

const USER_STORAGE_KEY = "yunji_user";

export interface StoredUser {
  nickname?: string;
  faceimg?: string;
  avatar?: string;
  user_id?: number;
  social_uid?: string;
  role?: string;
  plan?: string;
  unread_count?: number;
}

export interface UserInfo {
  name: string;
  avatar?: string;
  role: string;
  userId: string;
  plan: string;
  unreadCount: number;
}

export interface AuthState {
  user: UserInfo | null;
  loginModalOpen: boolean;
  loginModalContext?: string;
  loadUser: () => void;
  logout: () => void;
  openLoginModal: (context?: string) => void;
  closeLoginModal: () => void;
}

export function getStoredUser(): UserInfo | null {
  try {
    const raw = localStorage.getItem(USER_STORAGE_KEY);
    if (!raw) return null;
    const parsed: StoredUser = JSON.parse(raw);
    return {
      name: parsed.nickname || "云集用户",
      avatar: parsed.faceimg || parsed.avatar,
      role: parsed.role || "体验版",
      userId: String(parsed.user_id || parsed.social_uid || ""),
      plan: parsed.plan || "免费",
      unreadCount: parsed.unread_count || 0,
    };
  } catch {
    return null;
  }
}

export function clearStoredUser() {
  localStorage.removeItem(USER_STORAGE_KEY);
}

export function storeUser(user: StoredUser) {
  localStorage.setItem(USER_STORAGE_KEY, JSON.stringify(user));
  window.dispatchEvent(new Event("yunji:user-changed"));
}

export const useAuth = create<AuthState>((set) => ({
  user: getStoredUser(),
  loginModalOpen: false,
  loginModalContext: undefined,
  loadUser: () => set({ user: getStoredUser() }),
  logout: () => {
    clearStoredUser();
    set({ user: null });
  },
  openLoginModal: (context) =>
    set({ loginModalOpen: true, loginModalContext: context }),
  closeLoginModal: () => set({ loginModalOpen: false }),
}));

// 同步多个标签页的登录状态
window.addEventListener("storage", () => {
  useAuth.getState().loadUser();
});
window.addEventListener("yunji:user-changed", () => {
  useAuth.getState().loadUser();
});
