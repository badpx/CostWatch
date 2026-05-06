import { describe, it, expect } from "vitest";
import { rangeToMs, formatVal, getCurrencySymbol } from "../chart";
import type { ProviderState } from "../../types";

describe("rangeToMs", () => {
  it("converts 24h to milliseconds", () => {
    expect(rangeToMs("24h")).toBe(24 * 60 * 60 * 1000);
  });

  it("converts 1w to milliseconds", () => {
    expect(rangeToMs("1w")).toBe(7 * 24 * 60 * 60 * 1000);
  });

  it("converts 1m to milliseconds", () => {
    expect(rangeToMs("1m")).toBe(30 * 24 * 60 * 60 * 1000);
  });

  it("defaults to 1 week for unknown ranges", () => {
    expect(rangeToMs("unknown")).toBe(7 * 24 * 60 * 60 * 1000);
  });
});

describe("getCurrencySymbol", () => {
  it("returns $ for USD", () => {
    expect(getCurrencySymbol("USD")).toBe("$");
  });

  it("returns ¥ for CNY", () => {
    expect(getCurrencySymbol("CNY")).toBe("¥");
  });

  it("returns € for EUR", () => {
    expect(getCurrencySymbol("EUR")).toBe("€");
  });

  it("returns empty string for unknown string currency", () => {
    expect(getCurrencySymbol("GBP" as ProviderState["currency"])).toBe("");
  });

  it("returns custom symbol for Custom object", () => {
    expect(getCurrencySymbol({ Custom: "₩" })).toBe("₩");
  });

  it("returns empty string for invalid input", () => {
    expect(getCurrencySymbol(null as unknown as ProviderState["currency"])).toBe("");
  });
});

describe("formatVal", () => {
  it("formats small values with 2 decimals", () => {
    expect(formatVal(5.42, "USD")).toBe("$5.42");
  });

  it("formats values >= 10 with 0 decimals", () => {
    expect(formatVal(42, "USD")).toBe("$42");
  });

  it("formats large values with k suffix", () => {
    expect(formatVal(1500, "USD")).toBe("$1.5k");
  });

  it("uses provider-specific currency symbol", () => {
    expect(formatVal(100, "CNY")).toBe("¥100");
    expect(formatVal(100, "EUR")).toBe("€100");
  });

  it("uses custom currency symbol", () => {
    expect(formatVal(2500, { Custom: "₩" })).toBe("₩2.5k");
  });

  it("handles negative values correctly", () => {
    expect(formatVal(-5.5, "USD")).toBe("$-5.50");
  });
});
