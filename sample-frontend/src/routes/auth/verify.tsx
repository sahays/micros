import { createFileRoute, useNavigate } from "@tanstack/react-router";
import { useQuery } from "@tanstack/react-query";
import { useEffect } from "react";
import { AuthCard } from "@/components/auth/AuthCard";
import { CheckCircle2, XCircle, Loader2 } from "lucide-react";
import { authService } from "@/services/authService";
import axios from "axios";

export const Route = createFileRoute("/auth/verify")({
  component: VerifyPage,
  validateSearch: (search: Record<string, unknown>) => {
    return {
      token: (search.token as string) || "",
    };
  },
});

function VerifyPage() {
  const { token } = Route.useSearch();
  const navigate = useNavigate();

  const { data, isLoading, error } = useQuery({
    queryKey: ["verify-email", token],
    queryFn: () => authService.verifyEmail(token),
    enabled: !!token,
    retry: false,
  });

  useEffect(() => {
    if (data) {
      // Redirect to login after 2 seconds
      const timer = setTimeout(() => {
        navigate({ to: "/auth/login" });
      }, 2000);
      return () => clearTimeout(timer);
    }
  }, [data, navigate]);

  if (!token) {
    return (
      <AuthCard>
        <div className="text-center space-y-4 py-8">
          <div className="flex justify-center">
            <div className="flex size-16 items-center justify-center rounded-full bg-error/10">
              <XCircle className="size-8 text-error" />
            </div>
          </div>
          <div className="space-y-2">
            <h2 className="text-2xl font-bold text-sidebar-foreground">
              Invalid Verification Link
            </h2>
            <p className="text-sm text-muted-foreground">
              No verification token provided.
            </p>
          </div>
        </div>
      </AuthCard>
    );
  }

  if (isLoading) {
    return (
      <AuthCard>
        <div className="text-center space-y-4 py-8">
          <div className="flex justify-center">
            <Loader2 className="size-12 animate-spin text-primary" />
          </div>
          <div className="space-y-2">
            <h2 className="text-2xl font-bold text-sidebar-foreground">
              Verifying Your Email
            </h2>
            <p className="text-sm text-muted-foreground">
              Please wait while we verify your email address...
            </p>
          </div>
        </div>
      </AuthCard>
    );
  }

  if (error) {
    return (
      <AuthCard>
        <div className="text-center space-y-4 py-8">
          <div className="flex justify-center">
            <div className="flex size-16 items-center justify-center rounded-full bg-error/10">
              <XCircle className="size-8 text-error" />
            </div>
          </div>
          <div className="space-y-2">
            <h2 className="text-2xl font-bold text-sidebar-foreground">
              Verification Failed
            </h2>
            <p className="text-sm text-muted-foreground">
              {axios.isAxiosError(error)
                ? error.response?.data?.message ||
                  "Invalid or expired verification token."
                : "Invalid or expired verification token."}
            </p>
            <p className="text-sm text-muted-foreground pt-4">
              Please request a new verification email.
            </p>
          </div>
        </div>
      </AuthCard>
    );
  }

  return (
    <AuthCard>
      <div className="text-center space-y-4 py-8">
        <div className="flex justify-center">
          <div className="flex size-16 items-center justify-center rounded-full bg-success/10">
            <CheckCircle2 className="size-8 text-success" />
          </div>
        </div>
        <div className="space-y-2">
          <h2 className="text-2xl font-bold text-sidebar-foreground">
            Email Verified!
          </h2>
          <p className="text-sm text-muted-foreground">
            Your email has been successfully verified.
          </p>
          <p className="text-sm text-muted-foreground">
            Redirecting to login page...
          </p>
        </div>
      </div>
    </AuthCard>
  );
}
