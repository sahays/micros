import { useAuthStore } from "@/stores/useAuthStore";
import { useNavigate } from "@tanstack/react-router";

export function useAuth() {
  const navigate = useNavigate();
  const {
    user,
    accessToken,
    isAuthenticated,
    isLoading,
    login,
    logout: storeLogout,
    setUser,
    setLoading,
  } = useAuthStore();

  const logout = () => {
    storeLogout();
    navigate({ to: "/auth/login" });
  };

  const requireAuth = () => {
    if (!isAuthenticated && !isLoading) {
      navigate({ to: "/auth/login" });
      return false;
    }
    return isAuthenticated;
  };

  return {
    user,
    accessToken,
    isAuthenticated,
    isLoading,
    login,
    logout,
    setUser,
    setLoading,
    requireAuth,
  };
}
