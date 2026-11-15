/**
 * RF-21, RF-89: what the window knows about the disk. A command answer only says the change was applied and queued;
 * the file is written when the core sends `vault-save-state: saved`. `saved` counts those events (and a retry or a
 * "save elsewhere" that succeeded), so a notice can wait for the next one instead of trusting an old one. Nothing in
 * it is a vault datum.
 */
export interface SaveSignal {
  readonly saved: number;
  /** A failed save or a change on disk: the save notice of the open vault says what happened and what to do. */
  readonly problem: boolean;
}

export const NO_SAVE_SIGNAL: SaveSignal = { saved: 0, problem: false };

/** The notice of a change applied when `saved` was `since`: `waiting` until the next save lands, then `done`. */
export function savingText(signal: SaveSignal, since: number, waiting: string, done: string): string {
  if (signal.problem) return "";
  return signal.saved > since ? done : waiting;
}
