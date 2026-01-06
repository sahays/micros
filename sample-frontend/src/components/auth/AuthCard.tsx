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
    <div className="flex min-h-screen items-center justify-center bg-background p-4">
      {/* Background Pattern */}
      <div className="fixed inset-0 -z-10 overflow-hidden">
        <div className="absolute inset-0 bg-[linear-gradient(to_right,#80808012_1px,transparent_1px),linear-gradient(to_bottom,#80808012_1px,transparent_1px)] bg-[size:24px_24px]" />
        <div className="absolute left-0 right-0 top-0 -z-10 m-auto h-[500px] w-[500px] rounded-full bg-primary/10 blur-[150px]" />
      </div>

      <Card
        className={cn(
          "w-full max-w-md bg-card border-border material-shadow",
          className,
        )}
      >
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
              <h1 className="text-2xl font-bold text-foreground">{title}</h1>
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
