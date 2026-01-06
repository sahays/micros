import { createFileRoute, Link, useNavigate } from "@tanstack/react-router";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { useMutation } from "@tanstack/react-query";
import { AuthCard } from "@/components/auth/AuthCard";
import { PasswordInput } from "@/components/auth/PasswordInput";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Separator } from "@/components/ui/separator";
import {
  Form,
  FormControl,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from "@/components/ui/form";
import { registerSchema, type RegisterFormData } from "@/lib/validations/auth";
import { authService } from "@/services/authService";
import { useState } from "react";
import { AlertCircle, CheckCircle2, Loader2 } from "lucide-react";

export const Route = createFileRoute("/auth/register")({
  component: RegisterPage,
});

function RegisterPage() {
  const navigate = useNavigate();
  const [error, setError] = useState<string>("");
  const [success, setSuccess] = useState(false);
  const [isGoogleLoading, setIsGoogleLoading] = useState(false);

  const form = useForm<RegisterFormData>({
    resolver: zodResolver(registerSchema),
    defaultValues: {
      email: "",
      password: "",
      confirmPassword: "",
      name: "",
    },
  });

  const registerMutation = useMutation({
    mutationFn: authService.register,
    onSuccess: () => {
      setSuccess(true);
      // Redirect to verification notice after 3 seconds
      setTimeout(() => {
        navigate({ to: "/auth/login" });
      }, 3000);
    },
    onError: (err: any) => {
      const message =
        err.response?.data?.message || "Registration failed. Please try again.";
      setError(message);
    },
  });

  const onSubmit = (data: RegisterFormData) => {
    setError("");
    registerMutation.mutate(data);
  };

  const handleGoogleSignIn = async () => {
    try {
      setIsGoogleLoading(true);
      setError("");

      const { authorization_url, state, code_verifier } =
        await authService.initiateGoogleOAuth();

      // Store state and code_verifier in sessionStorage
      sessionStorage.setItem("oauth_state", state);
      sessionStorage.setItem("oauth_code_verifier", code_verifier);

      // Redirect to Google OAuth
      window.location.href = authorization_url;
    } catch (err: any) {
      setIsGoogleLoading(false);
      const message =
        err.response?.data?.message || "Failed to initiate Google sign-in";
      setError(message);
    }
  };

  if (success) {
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
              Check Your Email
            </h2>
            <p className="text-sm text-muted-foreground">
              We've sent a verification link to{" "}
              <span className="font-medium text-sidebar-foreground">
                {form.getValues("email")}
              </span>
            </p>
            <p className="text-sm text-muted-foreground">
              Click the link in the email to verify your account.
            </p>
          </div>
          <div className="pt-4">
            <p className="text-xs text-muted-foreground">
              Redirecting to login page...
            </p>
          </div>
        </div>
      </AuthCard>
    );
  }

  return (
    <AuthCard
      title="Create Account"
      description="Sign up to get started with MaterialM"
    >
      <Form {...form}>
        <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-4">
          {error && (
            <div className="flex items-center gap-2 rounded-lg bg-error/10 p-3 text-sm text-error">
              <AlertCircle className="size-4 shrink-0" />
              <p>{error}</p>
            </div>
          )}

          <FormField
            control={form.control}
            name="name"
            render={({ field }) => (
              <FormItem>
                <FormLabel>Name (Optional)</FormLabel>
                <FormControl>
                  <Input placeholder="John Doe" {...field} />
                </FormControl>
                <FormMessage />
              </FormItem>
            )}
          />

          <FormField
            control={form.control}
            name="email"
            render={({ field }) => (
              <FormItem>
                <FormLabel>Email</FormLabel>
                <FormControl>
                  <Input
                    type="email"
                    placeholder="you@example.com"
                    {...field}
                  />
                </FormControl>
                <FormMessage />
              </FormItem>
            )}
          />

          <FormField
            control={form.control}
            name="password"
            render={({ field }) => (
              <FormItem>
                <FormLabel>Password</FormLabel>
                <FormControl>
                  <PasswordInput
                    placeholder="Create a strong password"
                    {...field}
                  />
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
                <FormLabel>Confirm Password</FormLabel>
                <FormControl>
                  <PasswordInput
                    placeholder="Confirm your password"
                    {...field}
                  />
                </FormControl>
                <FormMessage />
              </FormItem>
            )}
          />

          <Button
            type="submit"
            className="w-full"
            disabled={registerMutation.isPending || isGoogleLoading}
          >
            {registerMutation.isPending ? "Creating account..." : "Sign Up"}
          </Button>

          <Separator className="my-4" />

          <Button
            type="button"
            variant="outline"
            className="w-full"
            disabled={registerMutation.isPending || isGoogleLoading}
            onClick={handleGoogleSignIn}
          >
            {isGoogleLoading ? (
              <>
                <Loader2 className="size-4 mr-2 animate-spin" />
                Connecting to Google...
              </>
            ) : (
              <>
                <svg className="size-4 mr-2" viewBox="0 0 24 24">
                  <path
                    fill="currentColor"
                    d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92c-.26 1.37-1.04 2.53-2.21 3.31v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.09z"
                  />
                  <path
                    fill="currentColor"
                    d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z"
                  />
                  <path
                    fill="currentColor"
                    d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.07H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.93l2.85-2.22.81-.62z"
                  />
                  <path
                    fill="currentColor"
                    d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.07l3.66 2.84c.87-2.6 3.3-4.53 6.16-4.53z"
                  />
                </svg>
                Continue with Google
              </>
            )}
          </Button>

          <Separator className="my-4" />

          <div className="text-center text-sm text-muted-foreground">
            Already have an account?{" "}
            <Link
              to="/auth/login"
              className="text-primary font-medium hover:underline"
            >
              Sign in
            </Link>
          </div>
        </form>
      </Form>
    </AuthCard>
  );
}
