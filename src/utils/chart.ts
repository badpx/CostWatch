import type { ProviderState } from "../types";

export function rangeToMs(range: string): number {
  switch (range) {
    case "24h": return 24 * 60 * 60 * 1000;
    case "1w": return 7 * 24 * 60 * 60 * 1000;
    case "1m": return 30 * 24 * 60 * 60 * 1000;
    default: return 7 * 24 * 60 * 60 * 1000;
  }
}

export function formatVal(v: number, currency: ProviderState["currency"]): string {
  const sym = getCurrencySymbol(currency);
  if (Math.abs(v) >= 1000) return `${sym}${(v / 1000).toFixed(1)}k`;
  return `${sym}${v.toFixed(v < 10 ? 2 : 0)}`;
}

export function getCurrencySymbol(currency: ProviderState["currency"]): string {
  if (typeof currency === "string") {
    return currency === "USD" ? "$" : currency === "CNY" ? "¥" : currency === "EUR" ? "€" : "";
  }
  if (typeof currency === "object" && currency !== null && "Custom" in currency) return currency.Custom;
  return "";
}
