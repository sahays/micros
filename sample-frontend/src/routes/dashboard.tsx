import { createFileRoute } from "@tanstack/react-router";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { ProtectedRoute } from "@/components/auth/ProtectedRoute";
import { useAuth } from "@/hooks/useAuth";
import { Activity, Users, TrendingUp, Clock } from "lucide-react";

export const Route = createFileRoute("/dashboard")({
  component: Dashboard,
});

function Dashboard() {
  const { user } = useAuth();

  return (
    <ProtectedRoute>
      <div className="space-y-8">
        <div>
          <h1 className="text-4xl font-bold text-foreground">
            Welcome back, {user?.name || user?.email || "User"}!
          </h1>
          <p className="text-muted-foreground mt-3 text-lg">
            Here's what's happening with your account today
          </p>
        </div>

        <div className="grid grid-cols-1 gap-6 md:grid-cols-2 lg:grid-cols-4">
          <Card className="bg-card border-border hover:border-primary/30 transition-all">
            <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
              <CardTitle className="text-sm font-medium text-foreground">
                Total Activity
              </CardTitle>
              <div className="flex size-10 items-center justify-center rounded-lg bg-primary/10">
                <Activity className="size-5 text-primary" />
              </div>
            </CardHeader>
            <CardContent>
              <div className="text-3xl font-bold text-foreground">24</div>
              <p className="text-sm text-muted-foreground mt-1">
                <span className="text-primary font-medium">+12%</span> from last month
              </p>
            </CardContent>
          </Card>

          <Card className="bg-card border-border hover:border-primary/30 transition-all">
            <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
              <CardTitle className="text-sm font-medium text-foreground">
                Active Sessions
              </CardTitle>
              <div className="flex size-10 items-center justify-center rounded-lg bg-primary/10">
                <Users className="size-5 text-primary" />
              </div>
            </CardHeader>
            <CardContent>
              <div className="text-3xl font-bold text-foreground">1</div>
              <p className="text-sm text-muted-foreground mt-1">
                Current device
              </p>
            </CardContent>
          </Card>

          <Card className="bg-card border-border hover:border-primary/30 transition-all">
            <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
              <CardTitle className="text-sm font-medium text-foreground">
                Account Status
              </CardTitle>
              <div className="flex size-10 items-center justify-center rounded-lg bg-primary/10">
                <TrendingUp className="size-5 text-primary" />
              </div>
            </CardHeader>
            <CardContent>
              <div className="text-3xl font-bold text-foreground">
                {user?.verified ? "Verified" : "Pending"}
              </div>
              <p className="text-sm text-muted-foreground mt-1">
                Email verification status
              </p>
            </CardContent>
          </Card>

          <Card className="bg-card border-border hover:border-primary/30 transition-all">
            <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
              <CardTitle className="text-sm font-medium text-foreground">
                Last Login
              </CardTitle>
              <div className="flex size-10 items-center justify-center rounded-lg bg-primary/10">
                <Clock className="size-5 text-primary" />
              </div>
            </CardHeader>
            <CardContent>
              <div className="text-3xl font-bold text-foreground">Now</div>
              <p className="text-sm text-muted-foreground mt-1">
                Current session
              </p>
            </CardContent>
          </Card>
        </div>

        <div className="grid grid-cols-1 gap-6 md:grid-cols-2 lg:grid-cols-3">
          <Card className="bg-card border-border">
            <CardHeader>
              <CardTitle className="text-foreground">Quick Actions</CardTitle>
            </CardHeader>
            <CardContent className="space-y-2">
              <p className="text-muted-foreground">
                Manage your account settings and preferences
              </p>
            </CardContent>
          </Card>

          <Card className="bg-card border-border">
            <CardHeader>
              <CardTitle className="text-foreground">Recent Activity</CardTitle>
            </CardHeader>
            <CardContent>
              <p className="text-muted-foreground">
                No recent activity to display
              </p>
            </CardContent>
          </Card>

          <Card className="bg-card border-border">
            <CardHeader>
              <CardTitle className="text-foreground">Notifications</CardTitle>
            </CardHeader>
            <CardContent>
              <p className="text-muted-foreground">
                You're all caught up!
              </p>
            </CardContent>
          </Card>
        </div>
      </div>
    </ProtectedRoute>
  );
}
