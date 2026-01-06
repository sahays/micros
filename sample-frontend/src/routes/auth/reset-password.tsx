import { createFileRoute, useNavigate, Link } from "@tanstack/react-router";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { useMutation } from "@tanstack/react-query";
import { AuthCard } from "@/components/auth/AuthCard";
import { Button } from "@/components/ui/button";
import { PasswordInput } from "@/components/auth/PasswordInput";
import {
  Form,
  FormControl,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from "@/components/ui/form";
import {
  resetPasswordSchema,
  type ResetPasswordFormData,
} from "@/lib/validations/auth";
import { authService } from "@/services/authService";
import { useState, useEffect } from "react";
import { CheckCircle2, AlertCircle, XCircle } from "lucide-react";

export const Route = createFileRoute("/auth/reset-password")({
  component: ResetPasswordPage,
  validateSearch: (search: Record<string, unknown>) => {
    return {
      token: (search.token as string) || "",
    };
  },
});

function ResetPasswordPage() {
  const { token } = Route.useSearch();
  const navigate = useNavigate();
  const [success, setSuccess] = useState(false);

  const form = useForm<ResetPasswordFormData>({
    resolver: zodResolver(resetPasswordSchema),
    defaultValues: {
      password: "",
      confirmPassword: "",
    },
  });

  const mutation = useMutation({
    mutationFn: (data: ResetPasswordFormData) =>
      authService.resetPassword(token, data.password),
    onSuccess: () => {
      setSuccess(true);
      form.reset();
    },
  });

  useEffect(() => {
    if (success) {
      // Redirect to login after 3 seconds
      const timer = setTimeout(() => {
        navigate({ to: "/auth/login" });
      }, 3000);
      return () => clearTimeout(timer);
    }
  }, [success, navigate]);

  const onSubmit = (data: ResetPasswordFormData) => {
    mutation.mutate(data);
  };

  if (!token) {
    return (
      <AuthCard title="Invalid Reset Link">
        <div className="space-y-6">
          <div className="flex justify-center">
            <div className="flex size-16 items-center justify-center rounded-full bg-error/10">
              <XCircle className="size-8 text-error" />
            </div>
          </div>

          <div className="space-y-2 text-center">
            <p className="text-sm text-muted-foreground">
              This password reset link is invalid or missing a token.
            </p>
            <p className="text-sm text-muted-foreground">
              Please request a new password reset link.
            </p>
          </div>

          <Link to="/auth/forgot-password">
            <Button variant="outline" className="w-full">
              Request New Link
            </Button>
          </Link>
        </div>
      </AuthCard>
    );
  }

  if (success) {
    return (
      <AuthCard
        title="Password Reset Successful!"
        description="Your password has been successfully updated."
      >
        <div className="space-y-6">
          <div className="flex justify-center">
            <div className="flex size-16 items-center justify-center rounded-full bg-success/10">
              <CheckCircle2 className="size-8 text-success" />
            </div>
          </div>

          <div className="space-y-2 text-center">
            <p className="text-sm text-muted-foreground">
              You can now sign in with your new password.
            </p>
            <p className="text-sm text-muted-foreground">
              Redirecting to login page...
            </p>
          </div>

          <Link to="/auth/login">
            <Button className="w-full">Continue to Login</Button>
          </Link>
        </div>
      </AuthCard>
    );
  }

  return (
    <AuthCard
      title="Reset Your Password"
      description="Enter your new password below."
    >
      <div className="space-y-6">
        {mutation.isError && (
          <div className="flex items-center gap-2 rounded-lg bg-error/10 p-3 text-sm text-error">
            <AlertCircle className="size-4 shrink-0" />
            <p>
              {(mutation.error as any)?.response?.data?.message ||
                "Failed to reset password. The link may be expired."}
            </p>
          </div>
        )}

        <Form {...form}>
          <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-4">
            <FormField
              control={form.control}
              name="password"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>New Password</FormLabel>
                  <FormControl>
                    <PasswordInput placeholder="••••••••" {...field} />
                  </FormControl>
                  <FormMessage />
                </FormItem>
              )}
            />

            <FormField
              control={form.control}
              name="confirmPassword"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>Confirm New Password</FormLabel>
                  <FormControl>
                    <PasswordInput placeholder="••••••••" {...field} />
                  </FormControl>
                  <FormMessage />
                </FormItem>
              )}
            />

            <Button
              type="submit"
              className="w-full"
              disabled={mutation.isPending}
            >
              {mutation.isPending ? "Resetting..." : "Reset Password"}
            </Button>
          </form>
        </Form>

        <div className="text-center text-sm">
          <Link to="/auth/login">
            <Button variant="link" size="sm">
              Back to Login
            </Button>
          </Link>
        </div>
      </div>
    </AuthCard>
  );
}
