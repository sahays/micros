import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/dashboard")({
  component: Dashboard,
});

function Dashboard() {
  return (
    <div className="p-2">
      <h3>Dashboard Page</h3>
    </div>
  );
}
