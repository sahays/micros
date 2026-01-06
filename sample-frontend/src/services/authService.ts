import { apiClient } from "@/lib/api";
import type { LoginFormData, RegisterFormData } from "@/lib/validations/auth";

export interface LoginResponse {
  access_token: string;
  refresh_token: string;
  token_type: string;
  expires_in: number;
}

export interface RegisterResponse {
  user_id: string;
  message: string;
}

export interface User {
  id: string;
  email: string;
  name: string;
  verified: boolean;
  created_at: string;
}

export const authService = {
  async login(data: LoginFormData): Promise<LoginResponse> {
    const response = await apiClient.post<LoginResponse>("/auth/login", data);
    return response.data;
  },

  async register(data: RegisterFormData): Promise<RegisterResponse> {
    const response = await apiClient.post<RegisterResponse>("/auth/register", {
      email: data.email,
      password: data.password,
      name: data.name,
    });
    return response.data;
  },

  async verifyEmail(token: string): Promise<{ message: string }> {
    const response = await apiClient.get(`/auth/verify?token=${token}`);
    return response.data;
  },

  async getProfile(): Promise<User> {
    const response = await apiClient.get<User>("/users/me");
    return response.data;
  },

  async updateProfile(data: { name?: string; email?: string }): Promise<User> {
    const response = await apiClient.patch<User>("/users/me", data);
    return response.data;
  },

  async logout(): Promise<void> {
    await apiClient.post("/auth/logout");
  },

  async refreshToken(refreshToken: string): Promise<LoginResponse> {
    const response = await apiClient.post<LoginResponse>("/auth/refresh", {
      refresh_token: refreshToken,
    });
    return response.data;
  },

  async forgotPassword(email: string): Promise<{ message: string }> {
    const response = await apiClient.post("/auth/password-reset/request", {
      email,
    });
    return response.data;
  },

  async resetPassword(
    token: string,
    newPassword: string,
  ): Promise<{ message: string }> {
    const response = await apiClient.post("/auth/password-reset/confirm", {
      token,
      new_password: newPassword,
    });
    return response.data;
  },

  // Google OAuth
  async initiateGoogleOAuth(): Promise<{
    authorization_url: string;
    state: string;
    code_verifier: string;
  }> {
    const response = await apiClient.get("/auth/oauth/google/authorize");
    return response.data;
  },

  async completeGoogleOAuth(
    code: string,
    state: string,
    codeVerifier: string,
  ): Promise<LoginResponse> {
    const response = await apiClient.post<LoginResponse>(
      "/auth/oauth/google/callback",
      {
        code,
        state,
        code_verifier: codeVerifier,
      },
    );
    return response.data;
  },
};
