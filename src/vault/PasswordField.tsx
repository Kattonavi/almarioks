import { useEffect, useImperativeHandle, useRef, useState, type ReactNode, type Ref } from "react";

import type { VaultClient } from "../ipc/client";
import { take } from "./entryDraft";
import { strengthLabel } from "./format";

/** RF-70: wait this long after the last keystroke before asking the core for the strength. */
export const PASSWORD_STRENGTH_DELAY_MS = 150;

export interface PasswordFieldHandle {
  /** What is written, left where it is (to check it before sending). */
  peek(): string;
  /** Reads the password and empties the field: from here on it lives only in the caller's stack (ADR-0002). */
  take(): string;
}

interface PasswordFieldProps {
  readonly ref: Ref<PasswordFieldHandle>;
  readonly id: string;
  readonly client: Pick<VaultClient, "passwordStrength">;
  /** The help under the field, if it has one. */
  readonly hint: string | null;
  readonly invalid: boolean;
  readonly errorId: string;
  /** The field went from empty to written, or back. */
  readonly onWrittenChange: (written: boolean) => void;
  /** Notices and the error of the field. */
  readonly children?: ReactNode;
}

/**
 * The password of an entry (RF-41, RF-70). It is not React state: it lives in the `<input>` alone, is read right
 * before the call and goes away with the dialog.
 */
export function PasswordField({ ref, id, client, hint, invalid, errorId, onWrittenChange, children }: PasswordFieldProps) {
  const [score, setScore] = useState<number | null>(null);
  const [written, setWritten] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const strengthSeq = useRef(0);
  const strengthTimer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const mounted = useRef(false);

  useEffect(() => {
    mounted.current = true;
    const timer = strengthTimer;
    const seq = strengthSeq;
    return () => {
      mounted.current = false;
      clearTimeout(timer.current);
      seq.current += 1;
    };
  }, []);

  function cancelStrength() {
    clearTimeout(strengthTimer.current);
    strengthSeq.current += 1;
  }

  function mark(next: boolean) {
    setWritten(next);
    onWrittenChange(next);
  }

  useImperativeHandle(ref, () => ({
    peek: () => inputRef.current?.value ?? "",
    take: () => {
      clearTimeout(strengthTimer.current);
      strengthSeq.current += 1;
      setScore(null);
      setWritten(false);
      onWrittenChange(false);
      return take(inputRef.current);
    },
  }));

  function onInput() {
    cancelStrength();
    const mine = strengthSeq.current;
    const empty = (inputRef.current?.value ?? "") === "";
    mark(!empty);
    if (empty) {
      setScore(null);
      return;
    }
    strengthTimer.current = setTimeout(() => {
      const value = inputRef.current?.value ?? "";
      if (value === "") return;
      void client.passwordStrength(value).then((result) => {
        if (!mounted.current || mine !== strengthSeq.current) return;
        setScore(result.ok ? result.value.score : null);
      });
    }, PASSWORD_STRENGTH_DELAY_MS);
  }

  const hintId = `${id}-hint`;
  const strengthId = `${id}-strength`;

  return (
    <div className="entry-field">
      <label htmlFor={id}>Contraseña</label>
      <div className="password-box">
        <input
          ref={inputRef}
          id={id}
          type="password"
          autoComplete="new-password"
          autoCapitalize="off"
          autoCorrect="off"
          spellCheck={false}
          aria-invalid={invalid}
          aria-describedby={`${hint === null ? "" : `${hintId} `}${strengthId} ${errorId}`}
          onInput={onInput}
        />
      </div>
      <div id={strengthId} aria-live="polite" className="strength-live">
        {written && score !== null && (
          <p className="strength-row">
            <span className="strength-bar" data-score={score} aria-hidden="true">
              <span />
            </span>
            Fortaleza: {strengthLabel(score)}
          </p>
        )}
      </div>
      {hint !== null && (
        <p id={hintId} className="path">
          {hint}
        </p>
      )}
      {children}
    </div>
  );
}
