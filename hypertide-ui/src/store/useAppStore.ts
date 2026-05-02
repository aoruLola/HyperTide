import { create } from 'zustand';

interface AppState {
  apiKey: string;
  userId: string;
  setApiKey: (key: string) => void;
  setUserId: (id: string) => void;
}

export const useAppStore = create<AppState>((set) => ({
  apiKey: localStorage.getItem('api_key') || 'dev-master-key',
  userId: localStorage.getItem('user_id') || 'dev-user',
  setApiKey: (key) => {
    localStorage.setItem('api_key', key);
    set({ apiKey: key });
  },
  setUserId: (id) => {
    localStorage.setItem('user_id', id);
    set({ userId: id });
  },
}));
