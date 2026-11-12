import { useEffect, useImperativeHandle, useRef, useState, type ReactNode, type Ref } from "react";

import type { GenerateOptions, VaultClient } from "../ipc/client";
import { errorMessage, type AppError } from "../ipc/errors";
import { GenerateIcon, HideIcon, ShowIcon } from "../ui/icons";
import { take } from "./entryDraft";
import { strengthLabel } from "./format";

/** RF-70: wait this long after the last keystroke before asking the core for the strength. */
const PASSWORD_STRENGTH_DELAY_MS = 150;

/** RF-65: the length of a generated password until the owner changes it, and the range the core accepts. */
const DEFAULT_LENGTH = 20;
const MIN_LENGTH = 8;
/** The longest password the generator makes. The one place that says it: every text below is built from it. */
const MAX_LENGTH = 500;
/** The slider covers the lengths of every day; the box beside it takes any length up to `MAX_LENGTH`. */
const SLIDER_MAX = 64;
const LENGTH_BOX_NAME = `Longitud exacta, hasta ${String(MAX_LENGTH)}`;
const LENGTH_HELP = `El deslizante llega a ${String(SLIDER_MAX)}. En la casilla puedes escribir hasta ${String(MAX_LENGTH)}.`;
const LENGTH_ERROR = `Entre ${String(MIN_LENGTH)} y ${String(MAX_LENGTH)} caracteres.`;

type Kind = "lower" | "upper" | "digits" | "symbols";

const KINDS: readonly (readonly [Kind, string])[] = [
  ["lower", "Minúsculas"],
  ["upper", "Mayúsculas"],
  ["digits", "Números"],
  ["symbols", "Símbolos"],
];

/** RF-65: 20 characters and the four kinds. */
const DEFAULT_OPTIONS: GenerateOptions = {
  length: DEFAULT_LENGTH,
  lower: true,
  upper: true,
  digits: true,
  symbols: true,
  avoidLookalikes: false,
};

const IMPOSSIBLE: AppError = { code: "ReglasImposibles" };

/** RF-68: with no kind chosen there is nothing to ask the core for. (The length is checked where it is written.) */
function possible(options: GenerateOptions): boolean {
  return KINDS.some(([kind]) => options[kind]);
}

/**
 * RF-68: the length written in the box, or `null` when it is not a whole number in range. Read from the text itself:
 * `Number("")` is 0 and `parseInt("12.5")` is 12, and neither is what was written.
 */
function lengthOf(text: string): number | null {
  if (!/^\d+$/.test(text)) return null;
  const length = Number(text);
  return length >= MIN_LENGTH && length <= MAX_LENGTH ? length : null;
}

export interface PasswordFieldHandle {
  /** What is written, left where it is (to check it before sending). */
  peek(): string;
  /** Reads the password and empties the field: from here on it lives only in the caller's stack (ADR-0002). */
  take(): string;
}

interface PasswordFieldProps {
  readonly ref: Ref<PasswordFieldHandle>;
  readonly id: string;
  readonly client: Pick<VaultClient, "passwordStrength" | "generatePassword">;
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
 * The password of an entry, with the eye and the generator inside the field (RF-41, RF-65 to RF-70, RF-101). The
 * password is not React state: written or generated, it lives in the `<input>` alone, is read right before the call
 * and goes away with the dialog. Generating is always a command of the core.
 */
export function PasswordField({ ref, id, client, hint, invalid, errorId, onWrittenChange, children }: PasswordFieldProps) {
  const [score, setScore] = useState<number | null>(null);
  const [written, setWritten] = useState(false);
  const [visible, setVisible] = useState(false);
  const [optionsOpen, setOptionsOpen] = useState(false);
  const [options, setOptions] = useState<GenerateOptions>(DEFAULT_OPTIONS);
  const [generatorError, setGeneratorError] = useState<AppError | null>(null);
  /** What the length box says, as written. `options.length` only ever holds the last length that was valid. */
  const [lengthText, setLengthText] = useState(String(DEFAULT_LENGTH));
  const lengthValid = lengthOf(lengthText) !== null;
  /** RF-109: the core could not generate. It is said beside the field, which stays as it was. */
  const [fieldError, setFieldError] = useState<AppError | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const strengthSeq = useRef(0);
  const strengthTimer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const generateSeq = useRef(0);
  /** What is in the field came from the generator and nobody has typed over it since. */
  const generated = useRef(false);
  const mounted = useRef(false);

  useEffect(() => {
    mounted.current = true;
    const timer = strengthTimer;
    const strength = strengthSeq;
    const generation = generateSeq;
    return () => {
      mounted.current = false;
      clearTimeout(timer.current);
      strength.current += 1;
      generation.current += 1;
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
      generateSeq.current += 1;
      generated.current = false;
      setScore(null);
      setVisible(false);
      setWritten(false);
      onWrittenChange(false);
      return take(inputRef.current);
    },
  }));

  function onInput() {
    cancelStrength();
    // Typed by hand: the options no longer replace it, and a generation on its way is dropped.
    generated.current = false;
    generateSeq.current += 1;
    setFieldError(null);
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

  /** RF-101: asks the core for a password, fills the field with it and leaves it in sight. */
  function generate(rules: GenerateOptions, lengthOk = lengthValid) {
    generateSeq.current += 1;
    const mine = generateSeq.current;
    // RF-68: a length out of range is never sent. The box says why under itself, so the options are put in sight.
    if (!lengthOk) {
      setOptionsOpen(true);
      return;
    }
    if (!possible(rules)) {
      setGeneratorError(IMPOSSIBLE);
      setOptionsOpen(true);
      return;
    }
    setGeneratorError(null);
    setFieldError(null);
    void client.generatePassword(rules).then((result) => {
      if (!mounted.current || mine !== generateSeq.current) return;
      if (!result.ok) {
        // RF-109: the system gave no randomness. Nothing else changes: the field keeps what it had, shown or hidden,
        // and the options stay as they were.
        if (result.error.code === "GeneradorNoDisponible") {
          setFieldError(result.error);
          return;
        }
        setGeneratorError(result.error);
        setOptionsOpen(true);
        return;
      }
      cancelStrength();
      if (inputRef.current !== null) inputRef.current.value = result.value.password;
      generated.current = true;
      setScore(result.value.score);
      setVisible(true);
      mark(true);
    });
  }

  function changeOptions(next: GenerateOptions, lengthOk = lengthValid) {
    setOptions(next);
    if (generated.current) {
      generate(next, lengthOk);
      return;
    }
    // A password written by hand is never replaced by an option; an impossible rule is still said at once.
    setGeneratorError(possible(next) ? null : IMPOSSIBLE);
  }

  /** The length from the slider: every position is a whole length in range, so it works like any other option. */
  function slideLength(text: string) {
    setLengthText(text);
    const length = lengthOf(text);
    if (length === null) return;
    changeOptions({ ...options, length }, true);
  }

  /**
   * The length as it is typed in the box. It only sets the length: typing never generates. "501" passes through
   * "50", which is in range, and that must not replace what is in the field (RF-68; walkthrough on the installed
   * app). "Generar" or Enter generate with it.
   */
  function typeLength(text: string) {
    setLengthText(text);
    const length = lengthOf(text);
    // Half typed, empty or out of range: the options keep the last good length, and "Generar" refuses (RF-68).
    if (length === null) return;
    setOptions({ ...options, length });
  }

  const hintId = `${id}-hint`;
  const strengthId = `${id}-strength`;
  const optionsId = `${id}-options`;
  const lengthId = `${id}-length`;
  const exactId = `${id}-length-exact`;
  const lengthHelpId = `${id}-length-help`;
  const lengthErrorId = `${id}-length-error`;

  return (
    <div className="entry-field">
      <label htmlFor={id}>Contraseña</label>
      <div className="password-box">
        <input
          ref={inputRef}
          id={id}
          // Protected while it is typed too: `browserSurface.ts` cancels the browser's copy, cut and drag from here.
          data-secret=""
          type={visible ? "text" : "password"}
          autoComplete="new-password"
          autoCapitalize="off"
          autoCorrect="off"
          spellCheck={false}
          aria-invalid={invalid}
          aria-describedby={`${hint === null ? "" : `${hintId} `}${strengthId} ${errorId}`}
          onInput={onInput}
        />
        <button
          type="button"
          className="password-eye"
          aria-label={visible ? "Ocultar contraseña" : "Mostrar contraseña"}
          onClick={() => {
            setVisible((shown) => !shown);
          }}
        >
          {visible ? <HideIcon /> : <ShowIcon />}
        </button>
        <button
          type="button"
          className="btn-primary password-generate"
          onClick={() => {
            generate(options);
          }}
        >
          <GenerateIcon />
          Generar
        </button>
      </div>
      {/* RF-109: beside the field, not in the options: it is the field that could not be filled. */}
      {fieldError !== null && (
        <p role="alert" className="field-error">
          {errorMessage(fieldError)}
        </p>
      )}
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
      <button
        type="button"
        className="link-button"
        aria-expanded={optionsOpen}
        aria-controls={optionsOpen ? optionsId : undefined}
        onClick={() => {
          setOptionsOpen((open) => !open);
        }}
      >
        {optionsOpen ? "Ocultar opciones del generador" : "Opciones del generador"}
      </button>
      {optionsOpen && (
        <div
          id={optionsId}
          className="generator-options"
          onKeyDown={(event) => {
            // The options sit inside the entry form: Enter on one of them generates instead of saving the entry.
            if (event.key === "Enter" && event.target instanceof HTMLInputElement) {
              event.preventDefault();
              generate(options);
            }
          }}
        >
          <div className="generator-length">
            <label htmlFor={lengthId}>Longitud</label>
            {/* Past 64 the slider stays at its end: the box is what says the length. */}
            <input
              id={lengthId}
              type="range"
              min={MIN_LENGTH}
              max={SLIDER_MAX}
              step={1}
              value={Math.min(options.length, SLIDER_MAX)}
              onChange={(event) => {
                slideLength(event.target.value);
              }}
            />
            <input
              id={exactId}
              className="generator-length-exact"
              type="number"
              inputMode="numeric"
              min={MIN_LENGTH}
              max={MAX_LENGTH}
              step={1}
              value={lengthText}
              aria-label={LENGTH_BOX_NAME}
              aria-invalid={!lengthValid}
              aria-describedby={`${lengthErrorId} ${lengthHelpId}`}
              onChange={(event) => {
                typeLength(event.target.value);
              }}
            />
          </div>
          <p id={lengthErrorId} className="field-error generator-length-error" aria-live="polite">
            {lengthValid ? "" : LENGTH_ERROR}
          </p>
          <p id={lengthHelpId} className="generator-length-help">
            {LENGTH_HELP}
          </p>
          <div className="generator-kinds" role="group" aria-label="Caracteres de la contraseña generada">
            {KINDS.map(([kind, label]) => (
              <label key={kind}>
                <input
                  type="checkbox"
                  checked={options[kind]}
                  onChange={(event) => {
                    changeOptions({ ...options, [kind]: event.target.checked });
                  }}
                />
                {label}
              </label>
            ))}
            <label>
              <input
                type="checkbox"
                checked={options.avoidLookalikes}
                onChange={(event) => {
                  changeOptions({ ...options, avoidLookalikes: event.target.checked });
                }}
              />
              Evitar caracteres parecidos (0 O o 1 l I |)
            </label>
          </div>
          <p role="alert" className="field-error">
            {generatorError === null ? "" : errorMessage(generatorError)}
          </p>
        </div>
      )}
    </div>
  );
}
