import { createFileRoute, Link, useNavigate } from "@tanstack/react-router";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Shield, Zap, Lock, ArrowRight } from "lucide-react";
import { ThemeToggle } from "@/components/theme/ThemeToggle";
import { useAuth } from "@/hooks/useAuth";
import { useEffect } from "react";

export const Route = createFileRoute("/")({
  component: LandingPage,
});

function LandingPage() {
  const { isAuthenticated } = useAuth();
  const navigate = useNavigate();

  useEffect(() => {
    // Redirect to dashboard if already logged in
    if (isAuthenticated) {
      navigate({ to: "/dashboard" });
    }
  }, [isAuthenticated, navigate]);

  return (
    <div className="min-h-screen bg-background">
      {/* Background Pattern */}
      <div className="fixed inset-0 -z-10 overflow-hidden">
        <div className="absolute inset-0 bg-[linear-gradient(to_right,#80808012_1px,transparent_1px),linear-gradient(to_bottom,#80808012_1px,transparent_1px)] bg-[size:24px_24px]" />
        <div className="absolute left-0 right-0 top-0 -z-10 m-auto h-[500px] w-[500px] rounded-full bg-primary/10 blur-[150px]" />
      </div>

      {/* Navigation */}
      <nav className="border-b border-border bg-surface/80 backdrop-blur-sm sticky top-0 z-50">
        <div className="container mx-auto px-4 h-16 flex items-center justify-between">
          <div className="flex items-center gap-2">
            <div className="flex size-8 items-center justify-center rounded-md bg-primary text-primary-foreground text-lg font-bold">
              M
            </div>
            <span className="text-xl font-bold text-foreground">MaterialM</span>
          </div>
          <div className="flex items-center gap-3">
            <ThemeToggle />
            <Link to="/auth/login">
              <Button variant="ghost">Sign In</Button>
            </Link>
            <Link to="/auth/register">
              <Button className="bg-primary hover:bg-primary-hover">
                Get Started
              </Button>
            </Link>
          </div>
        </div>
      </nav>

      {/* Hero Section */}
      <section className="container mx-auto px-4 py-20 md:py-32">
        <div className="max-w-4xl mx-auto text-center space-y-8">
          <div className="inline-block rounded-full px-4 py-1.5 bg-primary/10 border border-primary/20 text-primary text-sm font-medium mb-4">
            ✨ Powered by AI & Modern Web Technologies
          </div>
          <h1 className="text-5xl md:text-7xl font-bold text-foreground leading-tight">
            Intelligent access to <span className="text-primary">1000+</span>
            <br />
            AI-powered services
          </h1>
          <p className="text-xl md:text-2xl text-muted-foreground max-w-2xl mx-auto">
            Secure authentication for AI applications. Enterprise-grade security
            with intelligent user management.
          </p>
          <div className="flex flex-col sm:flex-row gap-4 justify-center pt-6">
            <Link to="/auth/register">
              <Button
                size="lg"
                className="w-full sm:w-auto bg-primary hover:bg-primary-hover text-lg px-8 h-12"
              >
                Get Started <ArrowRight className="ml-2 size-5" />
              </Button>
            </Link>
            <Link to="/auth/login">
              <Button
                size="lg"
                variant="outline"
                className="w-full sm:w-auto text-lg px-8 h-12"
              >
                Sign In
              </Button>
            </Link>
          </div>
        </div>
      </section>

      {/* Features Section */}
      <section className="container mx-auto px-4 py-20">
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6 max-w-6xl mx-auto">
          <Card className="bg-card border-border hover:border-primary/30 transition-all group">
            <CardHeader>
              <div className="flex size-14 items-center justify-center rounded-xl bg-primary/10 mb-4 group-hover:bg-primary/20 transition-colors">
                <Shield className="size-7 text-primary" />
              </div>
              <CardTitle className="text-xl">
                Enterprise-grade security
              </CardTitle>
            </CardHeader>
            <CardContent>
              <p className="text-muted-foreground">
                HMAC request signing, JWT tokens, and intelligent rate limiting
                protect your AI services.
              </p>
            </CardContent>
          </Card>

          <Card className="bg-card border-border hover:border-primary/30 transition-all group">
            <CardHeader>
              <div className="flex size-14 items-center justify-center rounded-xl bg-primary/10 mb-4 group-hover:bg-primary/20 transition-colors">
                <Zap className="size-7 text-primary" />
              </div>
              <CardTitle className="text-xl">
                Lightning-fast authentication
              </CardTitle>
            </CardHeader>
            <CardContent>
              <p className="text-muted-foreground">
                Built with React 19 and modern web technologies for instant,
                seamless authentication experiences.
              </p>
            </CardContent>
          </Card>

          <Card className="bg-card border-border hover:border-primary/30 transition-all group">
            <CardHeader>
              <div className="flex size-14 items-center justify-center rounded-xl bg-primary/10 mb-4 group-hover:bg-primary/20 transition-colors">
                <Lock className="size-7 text-primary" />
              </div>
              <CardTitle className="text-xl">OAuth & social auth</CardTitle>
            </CardHeader>
            <CardContent>
              <p className="text-muted-foreground">
                Integrated Google OAuth with PKCE, plus email verification and
                password reset flows.
              </p>
            </CardContent>
          </Card>
        </div>
      </section>

      {/* Footer */}
      <footer className="border-t border-border mt-20 py-12 bg-surface/50">
        <div className="container mx-auto px-4">
          <div className="flex flex-col md:flex-row items-center justify-between gap-4">
            <div className="flex items-center gap-2">
              <div className="flex size-8 items-center justify-center rounded-md bg-primary text-primary-foreground text-lg font-bold">
                M
              </div>
              <span className="text-lg font-bold text-foreground">
                MaterialM
              </span>
            </div>
            <p className="text-sm text-muted-foreground">
              Built with React 19, TanStack Router, and Tailwind CSS v4
            </p>
          </div>
        </div>
      </footer>
    </div>
  );
}
