import { cn } from "@/lib/utils";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { CreateDatabaseWizard } from "@/components/database/create-wizard/CreateDatabaseWizard";

export function CreateDatabaseView({
  className,
  ...props
}: React.ComponentProps<"div">) {
  return (
    <div className={cn("flex flex-col gap-6", className)} {...props}>
      <Card className="w-full max-w-lg mx-auto">
        <CardHeader className="text-center">
          <CardTitle className="text-xl">Create New Database</CardTitle>
          <CardDescription>
            Set up a new password database to store your credentials securely.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <CreateDatabaseWizard />
        </CardContent>
      </Card>
    </div>
  );
}
