import { createFileRoute } from "@tanstack/react-router";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Avatar, AvatarFallback } from "@/components/ui/avatar";
import {
  Form,
  FormControl,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from "@/components/ui/form";
import { ProtectedRoute } from "@/components/auth/ProtectedRoute";
import { useAuth } from "@/hooks/useAuth";
import { updateProfileSchema, type UpdateProfileFormData } from "@/lib/validations/auth";
import { authService } from "@/services/authService";
import { useState } from "react";
import { CheckCircle2, AlertCircle, User as UserIcon } from "lucide-react";
import { cn } from "@/lib/utils";

export const Route = createFileRoute("/profile")({
  component: ProfilePage,
});

function ProfilePage() {
  const { user, setUser } = useAuth();
  const queryClient = useQueryClient();
  const [successMessage, setSuccessMessage] = useState("");
  const [errorMessage, setErrorMessage] = useState("");

  // Fetch fresh profile data
  const { data: profile } = useQuery({
    queryKey: ["profile"],
    queryFn: authService.getProfile,
    initialData: user,
  });

  const form = useForm<UpdateProfileFormData>({
    resolver: zodResolver(updateProfileSchema),
    values: {
      name: profile?.name || "",
      email: profile?.email || "",
    },
  });

  const updateMutation = useMutation({
    mutationFn: authService.updateProfile,
    onSuccess: (data) => {
      setUser(data);
      queryClient.setQueryData(["profile"], data);
      setSuccessMessage("Profile updated successfully!");
      setErrorMessage("");
      setTimeout(() => setSuccessMessage(""), 3000);
    },
    onError: (err: any) => {
      const message = err.response?.data?.message || "Failed to update profile";
      setErrorMessage(message);
      setSuccessMessage("");
    },
  });

  const onSubmit = (data: UpdateProfileFormData) => {
    setSuccessMessage("");
    setErrorMessage("");
    updateMutation.mutate(data);
  };

  const isDirty = form.formState.isDirty;

  return (
    <ProtectedRoute>
      <div className="space-y-8 max-w-2xl">
        <div>
          <h1 className="text-4xl font-bold text-foreground">Profile</h1>
          <p className="text-muted-foreground mt-3 text-lg">
            Manage your account information
          </p>
        </div>

        <Card className="bg-card border-border">
          <CardHeader>
            <CardTitle className="text-foreground">Profile Information</CardTitle>
          </CardHeader>
          <CardContent className="space-y-6">
            {/* Avatar Section */}
            <div className="flex items-center gap-4">
              <Avatar className="size-20">
                <AvatarFallback className="bg-primary text-primary-foreground text-2xl">
                  {profile?.name?.[0]?.toUpperCase() || profile?.email?.[0]?.toUpperCase() || <UserIcon />}
                </AvatarFallback>
              </Avatar>
              <div>
                <h3 className="font-semibold text-lg">{profile?.name || "User"}</h3>
                <p className="text-sm text-muted-foreground">{profile?.email}</p>
              </div>
            </div>

            {/* Success/Error Messages */}
            {successMessage && (
              <div className="flex items-center gap-2 rounded-lg bg-success/10 p-3 text-sm text-success">
                <CheckCircle2 className="size-4 shrink-0" />
                <p>{successMessage}</p>
              </div>
            )}

            {errorMessage && (
              <div className="flex items-center gap-2 rounded-lg bg-error/10 p-3 text-sm text-error">
                <AlertCircle className="size-4 shrink-0" />
                <p>{errorMessage}</p>
              </div>
            )}

            {/* Edit Form */}
            <Form {...form}>
              <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-4">
                <FormField
                  control={form.control}
                  name="name"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel>Name</FormLabel>
                      <FormControl>
                        <Input placeholder="Your name" {...field} />
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
                        <Input type="email" placeholder="you@example.com" {...field} />
                      </FormControl>
                      <FormMessage />
                    </FormItem>
                  )}
                />

                <div className="flex gap-3 pt-4">
                  <Button
                    type="submit"
                    disabled={!isDirty || updateMutation.isPending}
                  >
                    {updateMutation.isPending ? "Saving..." : "Save Changes"}
                  </Button>
                  <Button
                    type="button"
                    variant="outline"
                    disabled={!isDirty}
                    onClick={() => form.reset()}
                  >
                    Cancel
                  </Button>
                </div>
              </form>
            </Form>
          </CardContent>
        </Card>

        <Card className="bg-card border-border">
          <CardHeader>
            <CardTitle className="text-foreground">Account Details</CardTitle>
          </CardHeader>
          <CardContent className="space-y-3">
            <div className="flex justify-between items-center py-2 border-b">
              <span className="text-sm font-medium">User ID</span>
              <span className="text-sm text-muted-foreground">{profile?.id}</span>
            </div>
            <div className="flex justify-between items-center py-2 border-b">
              <span className="text-sm font-medium">Email Verified</span>
              <span className={cn(
                "text-sm font-medium",
                profile?.verified ? "text-success" : "text-warning"
              )}>
                {profile?.verified ? "Yes" : "Pending"}
              </span>
            </div>
            <div className="flex justify-between items-center py-2">
              <span className="text-sm font-medium">Member Since</span>
              <span className="text-sm text-muted-foreground">
                {profile?.created_at ? new Date(profile.created_at).toLocaleDateString() : "N/A"}
              </span>
            </div>
          </CardContent>
        </Card>
      </div>
    </ProtectedRoute>
  );
}
