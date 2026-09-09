export interface QuotaWindow {
  limitId: string;
  usedPercent: number;
  remainingPercent: number;
  windowDurationMins: number;
  resetsAt?: number;
}

export interface ManagedAccount {
  id: string;
  email: string;
  alias?: string;
  planType?: string;
  isActive: boolean;
  credentialState: "valid" | "expired" | "missing";
  fiveHour?: QuotaWindow;
  weekly?: QuotaWindow;
  extraLimits: QuotaWindow[];
  lastUpdatedAt?: number;
  lastError?: string;
}

export interface DashboardState {
  accounts: ManagedAccount[];
  codexRunning: boolean;
  refreshedAt?: number;
}

export type LoginProgress =
  | { status: "idle" }
  | { status: "waiting"; authUrl: string }
  | { status: "completed"; account: ManagedAccount }
  | { status: "failed"; message: string };

export interface WindowPlacement {
  horizontal: "left" | "right";
  vertical: "up" | "down";
}
