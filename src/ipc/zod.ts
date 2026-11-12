import { z } from "zod";

/**
 * Zod configured for the app CSP (`script-src 'self'`, no `unsafe-eval`).
 *
 * Without `jitless`, Zod probes `new Function("")` and compiles object parsers with it; under a strict CSP the probe
 * is swallowed but still reported as a `securitypolicyviolation`. Zod reads this flag when a schema is built, so every
 * schema of the app must import `z` from this module, never from "zod" directly.
 */
z.config({ jitless: true });

export { z };
