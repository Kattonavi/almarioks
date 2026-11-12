import { useEffect, useRef } from "react";

/** RF-29 and plan.md: the core measures inactivity; the window tells it about keyboard or mouse at most every 10 s. */
export const ACTIVITY_INTERVAL_MS = 10_000;

const EVENTS = ["keydown", "pointerdown", "pointermove", "wheel"] as const;

/**
 * Calls `send` on the first keyboard or mouse event and then at most once per `intervalMs` (leading edge, so the
 * core hears about activity right away). Listens only while `enabled`.
 */
export function useActivityPing(
  send: () => void,
  enabled: boolean,
  intervalMs: number = ACTIVITY_INTERVAL_MS,
  now: () => number = Date.now,
): void {
  const lastSent = useRef(Number.NEGATIVE_INFINITY);

  useEffect(() => {
    if (!enabled) return;
    const onActivity = () => {
      const current = now();
      if (current - lastSent.current < intervalMs) return;
      lastSent.current = current;
      send();
    };
    for (const name of EVENTS) window.addEventListener(name, onActivity, { capture: true, passive: true });
    return () => {
      for (const name of EVENTS) window.removeEventListener(name, onActivity, { capture: true });
    };
  }, [send, enabled, intervalMs, now]);
}
