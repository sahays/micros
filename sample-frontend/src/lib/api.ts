import axios, { AxiosInstance, InternalAxiosRequestConfig } from "axios";
import CryptoJS from "crypto-js";
import { useAuthStore } from "@/stores/useAuthStore";

// Environment configuration
const API_BASE_URL = import.meta.env.VITE_API_BASE_URL || "http://localhost:8080";
const CLIENT_ID = import.meta.env.VITE_CLIENT_ID || "";
const SIGNING_SECRET = import.meta.env.VITE_SIGNING_SECRET || "";

/**
 * Generate a cryptographically secure nonce
 */
function generateNonce(): string {
  return CryptoJS.lib.WordArray.random(16).toString(CryptoJS.enc.Hex);
}

/**
 * Generate HMAC-SHA256 signature for BFF request signing
 * @param timestamp - ISO 8601 timestamp
 * @param nonce - Random nonce
 * @param method - HTTP method
 * @param path - Request path
 * @param body - Request body (if any)
 */
function generateSignature(
  timestamp: string,
  nonce: string,
  method: string,
  path: string,
  body?: any,
): string {
  // Construct the signing payload
  // Format: {timestamp}:{nonce}:{method}:{path}:{body_hash}
  const bodyHash = body
    ? CryptoJS.SHA256(JSON.stringify(body)).toString(CryptoJS.enc.Hex)
    : "";
  const payload = `${timestamp}:${nonce}:${method.toUpperCase()}:${path}:${bodyHash}`;

  // Generate HMAC-SHA256 signature
  const signature = CryptoJS.HmacSHA256(payload, SIGNING_SECRET).toString(
    CryptoJS.enc.Hex,
  );

  return signature;
}

/**
 * Create and configure the Axios instance with request signing and auth
 */
function createApiClient(): AxiosInstance {
  const client = axios.create({
    baseURL: API_BASE_URL,
    headers: {
      "Content-Type": "application/json",
    },
    timeout: 30000,
  });

  // Request interceptor: Add auth token and request signing
  client.interceptors.request.use(
    (config: InternalAxiosRequestConfig) => {
      const { accessToken } = useAuthStore.getState();

      // Add Authorization header if token exists
      if (accessToken) {
        config.headers.Authorization = `Bearer ${accessToken}`;
      }

      // Generate request signing headers
      const timestamp = new Date().toISOString();
      const nonce = generateNonce();
      const method = config.method?.toUpperCase() || "GET";
      const path = config.url || "/";
      const body = config.data;

      const signature = generateSignature(timestamp, nonce, method, path, body);

      // Add BFF signing headers
      config.headers["X-Client-ID"] = CLIENT_ID;
      config.headers["X-Timestamp"] = timestamp;
      config.headers["X-Nonce"] = nonce;
      config.headers["X-Signature"] = signature;

      return config;
    },
    (error) => {
      return Promise.reject(error);
    },
  );

  // Response interceptor: Handle 401 errors and token refresh
  client.interceptors.response.use(
    (response) => {
      return response;
    },
    async (error) => {
      const originalRequest = error.config;

      // Handle 401 Unauthorized
      if (error.response?.status === 401 && !originalRequest._retry) {
        originalRequest._retry = true;

        // TODO: Implement token refresh logic
        // For now, just logout the user
        const { logout } = useAuthStore.getState();
        logout();

        // Optionally redirect to login
        // window.location.href = '/login';

        return Promise.reject(error);
      }

      return Promise.reject(error);
    },
  );

  return client;
}

// Export the configured API client
export const apiClient = createApiClient();

// Export types for API responses
export interface ApiResponse<T = any> {
  data: T;
  message?: string;
  success: boolean;
}

export interface ApiError {
  message: string;
  code?: string;
  details?: any;
}
