import { getVersion } from "@tauri-apps/api/app";
import { useEffect, useState, type SubmitEvent } from "react";

import { assertNever } from "../assertNever";
import type { VaultClient } from "../ipc/client";
import { errorMessage, type AppError } from "../ipc/errors";
import { AUTO_LOCK_MINUTES, type AutoLockMinutes } from "../ipc/schemas";
import { WarningIcon } from "../ui/icons";
import { UpdatePanel } from "../update/UpdatePanel";

export type SettingsClient = Pick<VaultClient, "settingsGet" | "settingsSet" | "prepareUpdate">;

/** RF-73. */
const CLIPBOARD_MIN = 10;
const CLIPBOARD_MAX = 120;

type Control = "autoLock" | "clipboard";

type LoadState =
  | { readonly kind: "loading" }
  | { readonly kind: "failed"; readonly error: AppError }
  | { readonly kind: "ready" };

type SaveState =
  | { readonly kind: "idle" }
  | { readonly kind: "saving" }
  | { readonly kind: "saved" }
  /** `control`: the error goes next to it; `null`: a general failure, below the form. */
  | { readonly kind: "failed"; readonly control: Control | null; readonly error: AppError | null };

type VersionState =
  | { readonly kind: "loading" }
  | { readonly kind: "ready"; readonly version: string }
  | { readonly kind: "error" };

const CONTROL_TEXT: Readonly<Record<Control, string>> = {
  autoLock: "Ese tiempo de bloqueo no se pudo guardar. Elige 1, 5, 15 o 30 minutos.",
  clipboard: `Escribe un número entero de segundos entre ${String(CLIPBOARD_MIN)} y ${String(CLIPBOARD_MAX)}.`,
};

function minutesText(minutes: AutoLockMinutes): string {
  return minutes === 1 ? "1 minuto" : `${String(minutes)} minutos`;
}

function isAutoLock(value: number): value is AutoLockMinutes {
  return (AUTO_LOCK_MINUTES as readonly number[]).includes(value);
}

/** The clipboard seconds as written, or `null` if they are not a whole number from 10 to 120. */
function clipboardSecondsOf(text: string): number | null {
  if (!/^\d{1,3}$/u.test(text.trim())) return null;
  const seconds = Number(text.trim());
  return seconds >= CLIPBOARD_MIN && seconds <= CLIPBOARD_MAX ? seconds : null;
}

function politeText(load: LoadState, save: SaveState): string {
  if (load.kind === "loading") return "Cargando los ajustes…";
  switch (save.kind) {
    case "saving":
      return "Guardando los ajustes…";
    case "saved":
      return "Ajustes guardados.";
    case "idle":
    case "failed":
      return "";
    default:
      return assertNever(save);
  }
}

interface SettingsPanelProps {
  readonly client: SettingsClient;
  /** The app version; Tauri's by default. */
  readonly readVersion?: () => Promise<string>;
  /** Both settings were stored: e.g. the open vault asks again how long is left before it locks (RF-29). */
  readonly onSaved?: () => void;
}

/**
 * P8 (RF-29, RF-73, RF-82): the two settings, the update panel (P9), the version and the licence. It holds nothing of
 * the vault, so it is the same with the vault open, locked or closed. Few things, finished.
 */
export function SettingsPanel({ client, readVersion = getVersion, onSaved }: SettingsPanelProps) {
  const [load, setLoad] = useState<LoadState>({ kind: "loading" });
  const [attempt, setAttempt] = useState(0);
  const [autoLock, setAutoLock] = useState<AutoLockMinutes>(5);
  const [clipboard, setClipboard] = useState("20");
  const [lastChanged, setLastChanged] = useState<Control>("autoLock");
  const [save, setSave] = useState<SaveState>({ kind: "idle" });
  const [version, setVersion] = useState<VersionState>({ kind: "loading" });

  useEffect(() => {
    let active = true;
    void client.settingsGet().then((result) => {
      if (!active) return;
      if (!result.ok) {
        setLoad({ kind: "failed", error: result.error });
        return;
      }
      setAutoLock(result.value.autoLockMinutes);
      setClipboard(String(result.value.clipboardSeconds));
      setLoad({ kind: "ready" });
    });
    return () => {
      active = false;
    };
  }, [client, attempt]);

  useEffect(() => {
    let active = true;
    readVersion().then(
      (value) => {
        if (active) setVersion({ kind: "ready", version: value });
      },
      () => {
        if (active) setVersion({ kind: "error" });
      },
    );
    return () => {
      active = false;
    };
  }, [readVersion]);

  async function onSubmit(event: SubmitEvent<HTMLFormElement>) {
    event.preventDefault();
    if (save.kind === "saving") return;
    const seconds = clipboardSecondsOf(clipboard);
    if (seconds === null) {
      setSave({ kind: "failed", control: "clipboard", error: null });
      document.getElementById("settings-clipboard")?.focus();
      return;
    }
    setSave({ kind: "saving" });
    const result = await client.settingsSet({ autoLockMinutes: autoLock, clipboardSeconds: seconds });
    if (result.ok) {
      setAutoLock(result.value.autoLockMinutes);
      setClipboard(String(result.value.clipboardSeconds));
      setSave({ kind: "saved" });
      onSaved?.();
      return;
    }
    // Both values passed the window's checks: an `ArgumentoInvalido` belongs to the control changed last.
    setSave(
      result.error.code === "ArgumentoInvalido"
        ? { kind: "failed", control: lastChanged, error: null }
        : { kind: "failed", control: null, error: result.error },
    );
  }

  const failedOn = (control: Control) => save.kind === "failed" && save.control === control;
  const text = politeText(load, save);

  return (
    <div className="settings">
      <section aria-labelledby="settings-title">
        <h3 id="settings-title">Ajustes</h3>
        <div aria-live="polite">{text !== "" && <p>{text}</p>}</div>
        {load.kind === "failed" && (
          <>
            <p role="alert" className="error">
              No se pudieron leer los ajustes. {errorMessage(load.error)}
            </p>
            <button
              type="button"
              onClick={() => {
                setLoad({ kind: "loading" });
                setAttempt((value) => value + 1);
              }}
            >
              Volver a cargar los ajustes
            </button>
          </>
        )}
        {load.kind === "ready" && (
          <form onSubmit={(event) => void onSubmit(event)} noValidate>
            <div className="field">
              <label htmlFor="settings-auto-lock">Bloquear la bóveda tras este tiempo sin usar la app</label>
              <select
                id="settings-auto-lock"
                value={autoLock}
                aria-invalid={failedOn("autoLock")}
                aria-describedby="settings-auto-lock-help settings-auto-lock-error"
                onChange={(event) => {
                  const value = Number(event.target.value);
                  if (!isAutoLock(value)) return;
                  setAutoLock(value);
                  setLastChanged("autoLock");
                  setSave({ kind: "idle" });
                }}
              >
                {AUTO_LOCK_MINUTES.map((minutes) => (
                  <option key={minutes} value={minutes}>
                    {minutesText(minutes)}
                  </option>
                ))}
              </select>
              <p id="settings-auto-lock-help" className="path">
                Cuenta también el tiempo con el equipo suspendido. No existe «nunca»: la bóveda siempre se bloquea
                sola.
              </p>
              <p id="settings-auto-lock-error" role={failedOn("autoLock") ? "alert" : undefined} className="error">
                {failedOn("autoLock") ? CONTROL_TEXT.autoLock : ""}
              </p>
            </div>
            <div className="field">
              <label htmlFor="settings-clipboard">Limpiar el portapapeles tras (segundos)</label>
              <input
                id="settings-clipboard"
                type="number"
                inputMode="numeric"
                min={CLIPBOARD_MIN}
                max={CLIPBOARD_MAX}
                step={1}
                value={clipboard}
                aria-invalid={failedOn("clipboard")}
                aria-describedby="settings-clipboard-help settings-clipboard-error"
                onChange={(event) => {
                  setClipboard(event.target.value);
                  setLastChanged("clipboard");
                  setSave({ kind: "idle" });
                }}
              />
              <p id="settings-clipboard-help" className="path">
                Entre {CLIPBOARD_MIN} y {CLIPBOARD_MAX} segundos. Se aplica al copiar una contraseña u otro campo
                protegido; AlmarioKS solo lo limpia si nadie copió otra cosa.
              </p>
              <p id="settings-clipboard-error" role={failedOn("clipboard") ? "alert" : undefined} className="error">
                {failedOn("clipboard") ? CONTROL_TEXT.clipboard : ""}
              </p>
            </div>
            {save.kind === "failed" && save.control === null && save.error !== null && (
              <p role="alert" className="error">
                No se guardaron los ajustes. {errorMessage(save.error)}
              </p>
            )}
            <div className="actions">
              <button type="submit" className="btn-primary" disabled={save.kind === "saving"}>
                Guardar ajustes
              </button>
            </div>
          </form>
        )}
      </section>

      <UpdatePanel
        beforeInstall={async () => {
          const result = await client.prepareUpdate();
          if (!result.ok) throw new Error(result.error.code);
        }}
      />

      <section aria-labelledby="about-title">
        <h3 id="about-title">Acerca de AlmarioKS</h3>
        <p aria-live="polite" className="num">
          {version.kind === "loading"
            ? "Leyendo la versión…"
            : version.kind === "ready"
              ? `Versión ${version.version}`
              : "No se pudo leer la versión."}
        </p>
        <p className="notice notice-icon">
          <WarningIcon />
          Versión de desarrollo: no la uses con tus contraseñas reales.
        </p>
        <p>
          Software libre con licencia GPL-3.0: puedes usarlo, estudiarlo, compartirlo y modificarlo. Tus bóvedas son
          archivos KDBX que también abren en KeePass 2 y KeePassXC.
        </p>
      </section>
    </div>
  );
}
