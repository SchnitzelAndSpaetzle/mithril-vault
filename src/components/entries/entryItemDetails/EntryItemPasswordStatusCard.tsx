import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import {
  AlertTriangle,
  CheckCircle2,
  Copy,
  Lock,
  ShieldAlert,
} from "lucide-react";

type PasswordStatus = "good" | "reused" | "breach" | "short" | "compromised";

interface EntryItemPasswordStatusCardProps {
  status?: PasswordStatus;
}

export function EntryItemPasswordStatusCard({
  status = "good",
}: EntryItemPasswordStatusCardProps) {
  const statusConfig = {
    good: {
      icon: CheckCircle2,
      iconColor: "text-green-500",
      title: "No issues found",
      description:
        "Your password is strong and hasn't been found in any known data breaches.",
      actionLabel: null,
      cardClass: "border-green-500/20 bg-green-500/5",
    },
    reused: {
      icon: Copy,
      iconColor: "text-yellow-500",
      title: "Password reused",
      description:
        "This password is used in multiple entries. Consider using a unique password for better security.",
      actionLabel: "Generate New Password",
      cardClass: "border-yellow-500/20 bg-yellow-500/5",
    },
    breach: {
      icon: ShieldAlert,
      iconColor: "text-red-500",
      title: "Found in data breach",
      description:
        "This password has been exposed in a known data breach. Change it immediately to protect your account.",
      actionLabel: "Change Password",
      cardClass: "border-red-500/20 bg-red-500/5",
    },
    short: {
      icon: AlertTriangle,
      iconColor: "text-orange-500",
      title: "Password too short",
      description:
        "Your password is shorter than recommended. Use at least 12 characters for better security.",
      actionLabel: "Generate Strong Password",
      cardClass: "border-orange-500/20 bg-orange-500/5",
    },
    compromised: {
      icon: Lock,
      iconColor: "text-red-500",
      title: "Password compromised",
      description:
        "This password may be compromised. We recommend changing it as soon as possible.",
      actionLabel: "Change Password",
      cardClass: "border-red-500/20 bg-red-500/5",
    },
  };

  const config = statusConfig[status];
  const Icon = config.icon;

  return (
    <Card size="sm" className={`w-full rounded-md ${config.cardClass}`}>
      <CardHeader>
        <div className="flex items-center gap-2">
          <Icon className={`h-5 w-5 ${config.iconColor}`} />
          <CardTitle>{config.title}</CardTitle>
        </div>
      </CardHeader>
      <CardContent>
        <p className="text-sm text-muted-foreground">{config.description}</p>
      </CardContent>
      {config.actionLabel && (
        <CardFooter>
          <Button variant="outline" size="sm" className="w-full">
            {config.actionLabel}
          </Button>
        </CardFooter>
      )}
    </Card>
  );
}
