import { VIRTUAL_ASSETS, VIRTUAL_SHARES } from "../solana/config";
import type { StrategyView } from "../solana/strategy";

/** Parse a decimal string into minor units. Returns null if malformed or over-precise. */
export function parseAmount(input: string, decimals: number): bigint | null {
  const t = input.trim();
  if (t === "" || t === ".") return null;
  if (!/^\d*\.?\d*$/.test(t)) return null;
  const [whole, frac = ""] = t.split(".");
  if (frac.length > decimals) return null;
  const w = whole === "" ? 0n : BigInt(whole);
  const f = frac === "" ? 0n : BigInt(frac.padEnd(decimals, "0"));
  return w * 10n ** BigInt(decimals) + f;
}

/**
 * Shares a deposit would mint, matching the program:
 * usdc * (shares + VIRTUAL_SHARES) / (nav + VIRTUAL_ASSETS), floored. The virtual
 * offset prices the empty strategy too, so there is no first-deposit special case.
 */
export function estimateSharesOut(usdcMinor: bigint, navMinor: bigint, totalShares: bigint): bigint {
  return (usdcMinor * (totalShares + VIRTUAL_SHARES)) / (navMinor + VIRTUAL_ASSETS);
}

export interface RedeemLeg {
  index: number;
  mint: string;
  amountMinor: bigint; // the asset's own minor units (6dp for the example assets)
}

export interface RedeemEstimate {
  usdcMinor: bigint;
  legs: RedeemLeg[];
}

/**
 * The in-kind slice a redemption pays out, matching withdraw's proportional math:
 * balance * shares / (totalShares + VIRTUAL_SHARES) per vault, so the virtual
 * shares' slice of every vault stays behind.
 */
export function estimateRedeem(sharesMinor: bigint, view: StrategyView): RedeemEstimate {
  if (view.totalShares === 0n || sharesMinor <= 0n) {
    return {
      usdcMinor: 0n,
      legs: view.assets.map((a) => ({ index: a.index, mint: a.mint.toBase58(), amountMinor: 0n })),
    };
  }
  const divisor = view.totalShares + VIRTUAL_SHARES;
  const usdcMinor = (view.usdcAmount * sharesMinor) / divisor;
  const legs = view.assets.map((a) => ({
    index: a.index,
    mint: a.mint.toBase58(),
    amountMinor: (a.vaultAmount * sharesMinor) / divisor,
  }));
  return { usdcMinor, legs };
}

/** Apply a bps slippage tolerance to a floor (e.g. minimum shares out). */
export function applyToleranceFloor(amount: bigint, toleranceBps: number): bigint {
  return (amount * BigInt(10_000 - toleranceBps)) / 10_000n;
}

/** Ungrouped decimal string for filling an input (no thousands separators). */
export function toAmountInput(minor: bigint, decimals = 6): string {
  const base = 10n ** BigInt(decimals);
  const whole = (minor / base).toString();
  const frac = (minor % base).toString().padStart(decimals, "0").replace(/0+$/, "");
  return frac ? `${whole}.${frac}` : whole;
}

/** Parse a percent string (e.g. "40" or "12.5") into basis points, or null if invalid. */
export function parsePercentToBps(input: string): number | null {
  const t = input.trim();
  if (t === "" || !/^\d*\.?\d*$/.test(t)) return null;
  const pct = Number(t);
  if (!Number.isFinite(pct)) return null;
  return Math.round(pct * 100);
}
