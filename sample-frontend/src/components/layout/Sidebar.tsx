import { Link, useRouterState } from "@tanstack/react-router";
import {
  Home,
  LayoutDashboard,
  User,
  Settings,
  LogOut,
  Shield,
} from "lucide-react";
import { cn } from "@/lib/utils";
import { Separator } from "@/components/ui/separator";

interface SidebarProps {
  expanded: boolean;
  className?: string;
}

const navigationItems = [
  { to: "/", label: "Home", icon: Home },
  { to: "/dashboard", label: "Dashboard", icon: LayoutDashboard },
  { to: "/profile", label: "Profile", icon: User },
  { to: "/admin", label: "Admin", icon: Shield },
  { to: "/settings", label: "Settings", icon: Settings },
];

export function Sidebar({ expanded, className }: SidebarProps) {
  const router = useRouterState();
  const currentPath = router.location.pathname;

  return (
    <aside
      className={cn(
        "fixed left-0 top-0 z-40 h-full bg-sidebar transition-all duration-300 sidebar-shadow",
        expanded ? "w-[270px]" : "w-20",
        className,
      )}
    >
      <div className="flex h-full flex-col">
        {/* Logo */}
        <div className="flex h-16 items-center px-6">
          {expanded ? (
            <h1 className="text-xl font-bold text-primary">MaterialM</h1>
          ) : (
            <div className="flex size-8 items-center justify-center rounded-md bg-primary text-primary-foreground font-bold">
              M
            </div>
          )}
        </div>

        <Separator />

        {/* Navigation */}
        <nav className="flex-1 space-y-1 p-4">
          {navigationItems.map((item) => {
            const Icon = item.icon;
            const isActive = currentPath === item.to;

            return (
              <Link
                key={item.to}
                to={item.to}
                className={cn(
                  "flex items-center gap-3 rounded-lg px-3 py-2.5 text-sm font-medium transition-all duration-200",
                  isActive
                    ? "bg-secondary text-primary"
                    : "text-sidebar-foreground hover:bg-muted",
                  !expanded && "justify-center",
                )}
              >
                <Icon className="size-5 shrink-0" />
                {expanded && <span>{item.label}</span>}
              </Link>
            );
          })}
        </nav>

        <Separator />

        {/* Logout */}
        <div className="p-4">
          <button
            className={cn(
              "flex w-full items-center gap-3 rounded-lg px-3 py-2.5 text-sm font-medium text-sidebar-foreground hover:bg-muted transition-all duration-200",
              !expanded && "justify-center",
            )}
          >
            <LogOut className="size-5 shrink-0" />
            {expanded && <span>Logout</span>}
          </button>
        </div>
      </div>
    </aside>
  );
}
