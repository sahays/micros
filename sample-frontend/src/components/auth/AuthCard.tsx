import { ReactNode } from "react";
import { Card, CardContent, CardHeader } from "@/components/ui/card";
import { cn } from "@/lib/utils";

interface AuthCardProps {
  children: ReactNode;
  title?: string;
  description?: string;
  className?: string;
}

export function AuthCard({
  children,
  title,
  description,
  className,
}: AuthCardProps) {
  return (
    <div className="flex min-h-screen items-center justify-center bg-gradient-to-br from-blue-50 to-purple-50 p-4">
      <Card className={cn("w-full max-w-md material-shadow", className)}>
        {(title || description) && (
          <CardHeader className="space-y-1 text-center">
            {title && (
              <div className="flex justify-center mb-4">
                <div className="flex size-12 items-center justify-center rounded-full bg-primary text-primary-foreground text-xl font-bold">
                  M
                </div>
              </div>
            )}
            {title && (
              <h1 className="text-2xl font-bold text-sidebar-foreground">
                {title}
              </h1>
            )}
            {description && (
              <p className="text-sm text-muted-foreground">{description}</p>
            )}
          </CardHeader>
        )}
        <CardContent className="pt-6">{children}</CardContent>
      </Card>
    </div>
  );
}
