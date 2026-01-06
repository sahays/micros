import { createRootRoute, Outlet, useRouterState } from "@tanstack/react-router";
import { TanStackRouterDevtools } from "@tanstack/router-devtools";
import { ReactQueryDevtools } from "@tanstack/react-query-devtools";
import { AppShell } from "@/components/layout/AppShell";

function RootComponent() {
  const router = useRouterState();
  const pathname = router.location.pathname;

  // Don't show AppShell on landing page and auth pages
  const showAppShell = !pathname.startsWith("/auth") && pathname !== "/";

  return (
    <>
      {showAppShell ? (
        <AppShell>
          <Outlet />
        </AppShell>
      ) : (
        <Outlet />
      )}
      <TanStackRouterDevtools />
      <ReactQueryDevtools />
    </>
  );
}

export const Route = createRootRoute({
  component: RootComponent,
});
