import { useEffect, useRef, useState } from "react";

import type { GenerateOptions, VaultClient } from "../ipc/client";
import { errorMessage, type AppError } from "../ipc/errors";
import { strengthLabel } from "./format";

export const DEFAULT_LENGTH = 20;
export const MIN_LENGTH = 8;
export const MAX_LENGTH = 128;

type Kind = "lower" | "upper" | "digits" | "symbols";

const KINDS: readonly (readonly [Kind, string])[] = [
  ["lower", "Minúsculas (a-z)"],
  ["upper", "Mayúsculas (A-Z)"],
  ["digits", "Dígitos (0-9)"],
  ["symbols", "Símbolos (!@#$…)"],
];

type GenState =
  | { readonly kind: "idle" }
  | { readonly kind: "generating" }
  | { readonly kind: "done"; readonly password: string; readonly score: number }
  | { readonly kind: "failed"; readonly error: AppError };

interface GeneratorProps {
  readonly client: Pick<VaultClient, "generatePassword">;
  /** "Usar esta contraseña": the form puts it in its password field and closes the generator. */
  readonly onUse: (password: string, score: number) => void;
  readonly onClose: () => void;
}

/**
 * P6 (RF-65 to RF-70). The generated password is shown in clear only while this panel is mounted: it lives in its
 * local state and goes away when it closes, when it is used or when the vault locks (ADR-0002, exception of RF-76).
 */
export function Generator({ client, onUse, onClose }: GeneratorProps) {
  const [length, setLength] = useState(String(DEFAULT_LENGTH));
  const [kinds, setKinds] = useState<Readonly<Record<Kind, boolean>>>({
    lower: true,
    upper: true,
    digits: true,
    symbols: true,
  });
  const [avoidLookalikes, setAvoidLookalikes] = useState(false);
  const [gen, setGen] = useState<GenState>({ kind: "idle" });
  const seq = useRef(0);
  const firstRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    firstRef.current?.focus();
    const current = seq;
    return () => {
      current.current += 1;
    };
  }, []);

  async function generate() {
    const parsed = Number(length);
    const options: GenerateOptions = { length: parsed, ...kinds, avoidLookalikes };
    // RF-68: nothing is asked for with impossible rules.
    if (!Number.isInteger(parsed) || parsed < MIN_LENGTH || parsed > MAX_LENGTH || !Object.values(kinds).some(Boolean)) {
      setGen({ kind: "failed", error: { code: "ReglasImposibles" } });
      return;
    }
    seq.current += 1;
    const mine = seq.current;
    setGen({ kind: "generating" });
    const result = await client.generatePassword(options);
    if (mine !== seq.current) return;
    setGen(result.ok ? { kind: "done", ...result.value } : { kind: "failed", error: result.error });
  }

  return (
    <fieldset className="generator">
      <legend>Generador de contraseñas</legend>
      <div className="field">
        <label htmlFor="gen-length">
          Longitud (entre {MIN_LENGTH} y {MAX_LENGTH})
        </label>
        <input
          ref={firstRef}
          id="gen-length"
          type="number"
          inputMode="numeric"
          min={MIN_LENGTH}
          max={MAX_LENGTH}
          step={1}
          value={length}
          onChange={(event) => {
            setLength(event.target.value);
          }}
        />
      </div>
      <fieldset>
        <legend>Tipos de caracteres</legend>
        {KINDS.map(([kind, label]) => (
          <div key={kind}>
            <input
              type="checkbox"
              id={`gen-${kind}`}
              checked={kinds[kind]}
              onChange={(event) => {
                const checked = event.target.checked;
                setKinds((current) => ({ ...current, [kind]: checked }));
              }}
            />{" "}
            <label htmlFor={`gen-${kind}`}>{label}</label>
          </div>
        ))}
      </fieldset>
      <div>
        <input
          type="checkbox"
          id="gen-avoid"
          checked={avoidLookalikes}
          onChange={(event) => {
            setAvoidLookalikes(event.target.checked);
          }}
        />{" "}
        <label htmlFor="gen-avoid">Evitar caracteres parecidos (0 O o 1 l I |)</label>
      </div>
      <div className="actions">
        <button type="button" onClick={() => void generate()}>
          {gen.kind === "done" ? "Generar otra" : "Generar"}
        </button>
        <button type="button" onClick={onClose}>
          Cerrar generador
        </button>
      </div>
      <div aria-live="polite">
        {gen.kind === "generating" && <p role="status">Generando…</p>}
        {gen.kind === "done" && (
          <>
            <p>
              Contraseña generada: <output className="secret">{gen.password}</output>
            </p>
            <p>
              Fortaleza: {strengthLabel(gen.score)} ({gen.score} de 4){" "}
              <meter min={0} max={4} low={2} high={3} optimum={4} value={gen.score} aria-label="Fortaleza" />
            </p>
          </>
        )}
      </div>
      {gen.kind === "failed" && (
        <p role="alert" className="error">
          {errorMessage(gen.error)}
        </p>
      )}
      {gen.kind === "done" && (
        <div className="actions">
          <button
            type="button"
            onClick={() => {
              onUse(gen.password, gen.score);
            }}
          >
            Usar esta contraseña
          </button>
        </div>
      )}
    </fieldset>
  );
}
