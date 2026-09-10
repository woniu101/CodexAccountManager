const PLAN_LABELS: Record<string, string> = {
  free: "FREE",
  go: "GO",
  plus: "PLUS",
  pro: "PRO",
  business: "BUSINESS",
  team: "BUSINESS",
  enterprise: "ENTERPRISE",
  edu: "EDU",
  education: "EDU",
};

export function planLabel(planType?: string): string {
  const normalized = planType?.trim().toLowerCase();
  if (!normalized) return "CHATGPT";
  return PLAN_LABELS[normalized] ?? normalized.replace(/[\s_-]+/g, " ").toUpperCase();
}
