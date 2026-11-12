import { cleanup, fireEvent, render } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { ACTIVITY_INTERVAL_MS, useActivityPing } from "./useActivityPing";

function Probe({ send, enabled, now }: { send: () => void; enabled: boolean; now: () => number }) {
  useActivityPing(send, enabled, ACTIVITY_INTERVAL_MS, now);
  return null;
}

afterEach(cleanup);

describe("activity ping", () => {
  it("RF-29: tells the core about keyboard and mouse at most once every 10 s", () => {
    let clock = 1_000_000;
    const now = () => clock;
    const send = vi.fn();
    render(<Probe send={send} enabled now={now} />);

    fireEvent.keyDown(window, { key: "a" });
    expect(send).toHaveBeenCalledTimes(1);

    clock += 4_000;
    fireEvent.pointerMove(window);
    fireEvent.pointerDown(window);
    clock += 5_999;
    fireEvent.wheel(window);
    expect(send).toHaveBeenCalledTimes(1);

    clock += 1;
    fireEvent.pointerDown(window);
    expect(send).toHaveBeenCalledTimes(2);
  });

  it("RF-29: says nothing while disabled and stops listening on unmount", () => {
    const send = vi.fn();
    const now = () => 0;
    const { rerender, unmount } = render(<Probe send={send} enabled={false} now={now} />);
    fireEvent.keyDown(window, { key: "a" });
    expect(send).not.toHaveBeenCalled();

    rerender(<Probe send={send} enabled now={now} />);
    unmount();
    fireEvent.keyDown(window, { key: "a" });
    expect(send).not.toHaveBeenCalled();
  });
});
