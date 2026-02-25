import axios from 'axios';

// API 基础配置
const API_BASE_URL = import.meta.env.VITE_API_URL || 'http://localhost:3000';
const DEV_API_KEY = 'dev-master-key';

export const api = axios.create({
  baseURL: API_BASE_URL,
  headers: {
    'Content-Type': 'application/json',
  },
});

// 请求拦截器 - 添加 API Key
api.interceptors.request.use((config) => {
  const apiKey = localStorage.getItem('api_key') || DEV_API_KEY;
  if (apiKey) {
    config.headers['X-API-Key'] = apiKey;
  }
  return config;
});

// 响应拦截器 - 错误处理
api.interceptors.response.use(
  (response) => response,
  (error) => {
    console.error('API Error:', error.response?.data || error.message);
    return Promise.reject(error);
  }
);

// API 类型定义
export interface FileLock {
  file_path: string;
  owner_id: string;
  locked_at: string;
}

export interface StoredFile {
  hash: string;
  original_path: string;
  size_bytes: number;
  stored_at: string;
}

export interface ApiKey {
  key: string;
  owner_id: string;
  permissions: string[];
  created_at: string;
  expires_at: string | null;
  revoked: boolean;
}

// API 方法
export const apiClient = {
  // 健康检查
  health: () => api.get('/health'),

  // 锁定管理
  locks: {
    list: () => api.get<FileLock[]>('/api/locks'),
    lock: (file_path: string, owner_id: string) =>
      api.post<FileLock>('/api/lock', { file_path, owner_id }),
    unlock: (file_path: string, owner_id: string) =>
      api.delete('/api/unlock', { data: { file_path, owner_id } }),
    forceUnlock: (file_path: string) =>
      api.post('/api/break-lock', { file_path }),
  },

  // 存储管理
  storage: {
    upload: (file: File, original_path: string) => {
      const formData = new FormData();
      formData.append('file', file);
      formData.append('original_path', original_path);
      return api.post<StoredFile>('/api/upload', formData, {
        headers: { 'Content-Type': 'multipart/form-data' },
      });
    },
    download: (hash: string) =>
      api.get(`/api/download/${hash}`, { responseType: 'blob' }),
    exists: (hash: string) => api.get<{ exists: boolean }>(`/api/exists/${hash}`),
    calculateHash: (file: File) => {
      const formData = new FormData();
      formData.append('file', file);
      return api.post<{ hash: string }>('/api/hash', formData, {
        headers: { 'Content-Type': 'multipart/form-data' },
      });
    },
  },

  // 认证管理
  auth: {
    verify: (key: string) =>
      api.get('/api/auth/verify', { headers: { 'X-API-Key': key } }),
    generate: (owner_id: string, permissions: string[], expires_in_days?: number) =>
      api.post<ApiKey>('/api/auth/generate', { owner_id, permissions, expires_in_days }),
    revoke: (key: string) =>
      api.delete('/api/auth/revoke', { data: { key } }),
    listKeys: () => api.get<ApiKey[]>('/api/auth/keys'),
  },
};
