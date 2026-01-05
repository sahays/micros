import { ReactNode } from "react";
import { useLayoutStore } from "@/stores/useLayoutStore";
import { Sidebar } from "./Sidebar";
import { Header } from "./Header";
import { Sheet, SheetContent } from "@/components/ui/sheet";
import { cn } from "@/lib/utils";

interface AppShellProps {
  children: ReactNode;
}

export function AppShell({ children }: AppShellProps) {
  const {
    sidebarExpanded,
    mobileSidebarOpen,
    toggleSidebar,
    setMobileSidebarOpen,
  } = useLayoutStore();

  return (
    <div className="relative min-h-screen">
      {/* Desktop Sidebar */}
      <div className="hidden md:block">
        <Sidebar expanded={sidebarExpanded} />
      </div>

      {/* Mobile Sidebar (Sheet) */}
      <Sheet open={mobileSidebarOpen} onOpenChange={setMobileSidebarOpen}>
        <SheetContent side="left" className="p-0 w-[270px]">
          <Sidebar expanded={true} />
        </SheetContent>
      </Sheet>

      {/* Main Content Area */}
      <div
        className={cn(
          "transition-all duration-300",
          sidebarExpanded ? "md:ml-[270px]" : "md:ml-20",
        )}
      >
        {/* Header */}
        <Header
          onMenuClick={() => {
            // On mobile, toggle the sheet
            if (window.innerWidth < 768) {
              setMobileSidebarOpen(!mobileSidebarOpen);
            } else {
              // On desktop, toggle sidebar expansion
              toggleSidebar();
            }
          }}
          sidebarExpanded={sidebarExpanded}
        />

        {/* Page Content */}
        <main className="min-h-[calc(100vh-4rem)] bg-background p-6">
          {children}
        </main>
      </div>
    </div>
  );
}
