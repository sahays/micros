import { createFileRoute } from "@tanstack/react-router";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";

export const Route = createFileRoute("/settings")({
  component: Settings,
});

function Settings() {
  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-3xl font-bold text-sidebar-foreground">Settings</h1>
        <p className="text-muted-foreground mt-2">
          Configure application preferences
        </p>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Application Settings</CardTitle>
        </CardHeader>
        <CardContent>
          <p className="text-sm text-muted-foreground">
            Settings panel coming soon...
          </p>
        </CardContent>
      </Card>
    </div>
  );
}
