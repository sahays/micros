import axios, { AxiosInstance, InternalAxiosRequestConfig } from "axios";
import { useAuthStore } from "@/stores/useAuthStore";

// Environment configuration
const API_BASE_URL = "/api";

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

  // Request interceptor: Add auth token
  client.interceptors.request.use(
    (config: InternalAxiosRequestConfig) => {
      const { accessToken } = useAuthStore.getState();

      // Add Authorization header if token exists
      if (accessToken) {
        config.headers.Authorization = `Bearer ${accessToken}`;
      }

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

        try {
          // Try to refresh the token
          const refreshToken = localStorage.getItem("refresh_token");

          if (refreshToken) {
            const response = await axios.post(`${API_BASE_URL}/auth/refresh`, {
              refresh_token: refreshToken,
            });

            const { access_token, refresh_token: newRefreshToken } =
              response.data;

            // Update tokens in store and localStorage
            const { setUser } = useAuthStore.getState();
            localStorage.setItem("refresh_token", newRefreshToken);

            // Get updated user profile
            const profileResponse = await axios.get(
              `${API_BASE_URL}/users/me`,
              {
                headers: { Authorization: `Bearer ${access_token}` },
              },
            );

            setUser(profileResponse.data);

            // Retry original request with new token
            originalRequest.headers.Authorization = `Bearer ${access_token}`;
            return client(originalRequest);
          }
        } catch {
          // Refresh failed, logout user
          const { logout } = useAuthStore.getState();
          logout();
          localStorage.removeItem("refresh_token");

          if (typeof window !== "undefined") {
            window.location.href = "/auth/login";
          }
        }

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
export interface ApiResponse<T = unknown> {
  data: T;
  message?: string;
  success: boolean;
}

export interface ApiError {
  message: string;
  code?: string;
  details?: unknown;
}
