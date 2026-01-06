import { createFileRoute, Link } from "@tanstack/react-router";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { useMutation } from "@tanstack/react-query";
import { AuthCard } from "@/components/auth/AuthCard";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Form,
  FormControl,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from "@/components/ui/form";
import {
  forgotPasswordSchema,
  type ForgotPasswordFormData,
} from "@/lib/validations/auth";
import { authService } from "@/services/authService";
import { useState } from "react";
import { CheckCircle2, AlertCircle, ArrowLeft } from "lucide-react";

export const Route = createFileRoute("/auth/forgot-password")({
  component: ForgotPasswordPage,
});

function ForgotPasswordPage() {
  const [success, setSuccess] = useState(false);

  const form = useForm<ForgotPasswordFormData>({
    resolver: zodResolver(forgotPasswordSchema),
    defaultValues: {
      email: "",
    },
  });

  const mutation = useMutation({
    mutationFn: authService.forgotPassword,
    onSuccess: () => {
      setSuccess(true);
      form.reset();
    },
  });

  const onSubmit = (data: ForgotPasswordFormData) => {
    mutation.mutate(data.email);
  };

  if (success) {
    return (
      <AuthCard
        title="Check Your Email"
        description="If an account exists with that email, you'll receive password reset instructions."
      >
        <div className="space-y-6">
          <div className="flex justify-center">
            <div className="flex size-16 items-center justify-center rounded-full bg-success/10">
              <CheckCircle2 className="size-8 text-success" />
            </div>
          </div>

          <div className="space-y-2 text-center">
            <p className="text-sm text-muted-foreground">
              We've sent password reset instructions to your email address if it
              exists in our system.
            </p>
            <p className="text-sm text-muted-foreground">
              Please check your inbox and spam folder.
            </p>
          </div>

          <div className="space-y-3">
            <Link to="/auth/login">
              <Button variant="outline" className="w-full">
                <ArrowLeft className="size-4 mr-2" />
                Back to Login
              </Button>
            </Link>
            <Button
              variant="ghost"
              className="w-full"
              onClick={() => setSuccess(false)}
            >
              Try Another Email
            </Button>
          </div>
        </div>
      </AuthCard>
    );
  }

  return (
    <AuthCard
      title="Forgot Password?"
      description="Enter your email address and we'll send you instructions to reset your password."
    >
      <div className="space-y-6">
        {mutation.isError && (
          <div className="flex items-center gap-2 rounded-lg bg-error/10 p-3 text-sm text-error">
            <AlertCircle className="size-4 shrink-0" />
            <p>
              {(mutation.error as any)?.response?.data?.message ||
                "Something went wrong. Please try again."}
            </p>
          </div>
        )}

        <Form {...form}>
          <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-4">
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

            <Button
              type="submit"
              className="w-full"
              disabled={mutation.isPending}
            >
              {mutation.isPending ? "Sending..." : "Send Reset Link"}
            </Button>
          </form>
        </Form>

        <div className="text-center">
          <Link to="/auth/login">
            <Button variant="ghost" size="sm">
              <ArrowLeft className="size-4 mr-2" />
              Back to Login
            </Button>
          </Link>
        </div>
      </div>
    </AuthCard>
  );
}
