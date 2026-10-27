import type { MsmRequest, WorkerInbound, WorkerOutbound } from "./protocol";

export interface WorkerWasm {
  initSync(module: { module: BufferSource }): void;
  default?: () => Promise<unknown>;
  msmG1(bases: Uint8Array, scalars: Uint8Array): Uint8Array;
  msmG2(bases: Uint8Array, scalars: Uint8Array): Uint8Array;
}

const basesByKey = new Map<string, Uint8Array>();
let started = false;

/** Node worker loads WASM itself, then accepts MSM jobs. */
export function markWorkerStarted(): void {
  started = true;
}

export function handleWorkerMessage(
  wasm: WorkerWasm,
  message: WorkerInbound,
  post: (message: WorkerOutbound, transfer?: ArrayBuffer[]) => void,
): void {
  if (message.type === "init") {
    startWasm(wasm, message.wasm);
    post({ type: "ready" });
    return;
  }
  if (!started) {
    post({
      type: "error",
      id: message.id,
      message: "Groth16 worker is not initialized",
    });
    return;
  }
  try {
    const point = multiply(wasm, message);
    const copy = point.slice();
    post({ type: "result", id: message.id, point: copy.buffer }, [copy.buffer]);
  } catch (error) {
    post({
      type: "error",
      id: message.id,
      message: error instanceof Error ? error.message : String(error),
    });
  }
}

function startWasm(wasm: WorkerWasm, binary: ArrayBuffer | undefined): void {
  if (started) {
    return;
  }
  if (binary) {
    wasm.initSync({ module: binary });
  } else if (wasm.default) {
    throw new Error("Browser Groth16 worker needs the WASM bytes");
  } else {
    throw new Error("Groth16 worker has no WASM binary");
  }
  started = true;
}

function multiply(wasm: WorkerWasm, message: MsmRequest): Uint8Array {
  if (message.bases) {
    basesByKey.set(message.cacheKey, new Uint8Array(message.bases));
  }
  const bases = basesByKey.get(message.cacheKey);
  if (!bases) {
    throw new Error(`Missing MSM bases for ${message.cacheKey}`);
  }
  const scalars = new Uint8Array(message.scalars);
  return message.curve === "g1"
    ? wasm.msmG1(bases, scalars)
    : wasm.msmG2(bases, scalars);
}
