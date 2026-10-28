import type { ProverHost, ProverOutbound, ProverPort } from "./prover-protocol";

const START_TIMEOUT_MS = 60_000;

interface PendingCall {
  resolve: (message: ProverOutbound) => void;
  reject: (error: Error) => void;
}

const sessions = new Map<
  string,
  { host: ProverHost; bases: Promise<Uint8Array> }
>();

/** One worker and one export per proving key, shared by later SDK instances. */
export function sharedProver(
  spawn: () => ProverPort,
  provingKey: Uint8Array,
  r1cs: Uint8Array,
): { host: ProverHost; bases: Promise<Uint8Array> } {
  const id = `${fingerprint(provingKey)}:${fingerprint(r1cs)}`;
  const existing = sessions.get(id);
  if (existing) {
    return existing;
  }
  const host = createProverClient(spawn);
  const created = {
    host,
    bases: host.exportBases(provingKey, r1cs),
  };
  sessions.set(id, created);
  return created;
}

/** Serial calls into the worker that holds the proving key. */
export function createProverClient(spawn: () => ProverPort): ProverHost {
  let port: ProverPort | null = null;
  let ready: Promise<void> | null = null;
  let tail: Promise<void> = Promise.resolve();
  let nextId = 1;
  const pending = new Map<number, PendingCall>();
  let disposed = false;

  function enqueue<T>(run: () => Promise<T>): Promise<T> {
    const result = tail.then(run, run);
    tail = result.then(
      () => undefined,
      () => undefined,
    );
    return result;
  }

  function boot(): Promise<void> {
    if (ready) {
      return ready;
    }
    const worker = spawn();
    port = worker;
    ready = new Promise((resolve, reject) => {
      const timeout = setTimeout(() => {
        reject(new Error("Groth16 prover worker did not start"));
      }, START_TIMEOUT_MS);
      worker.fail((error) => {
        clearTimeout(timeout);
        failAll(error);
        reject(error);
      });
      worker.listen((message) => {
        if (message.type === "ready") {
          clearTimeout(timeout);
          worker.unref();
          resolve();
          return;
        }
        const waiter = pending.get(message.id);
        if (!waiter) {
          return;
        }
        pending.delete(message.id);
        if (pending.size === 0) {
          worker.unref();
        }
        if (message.type === "error") {
          waiter.reject(new Error(message.message));
          return;
        }
        waiter.resolve(message);
      });
      worker.post({ type: "init" });
    });
    return ready;
  }

  function failAll(error: Error): void {
    for (const waiter of pending.values()) {
      waiter.reject(error);
    }
    pending.clear();
    port?.unref();
  }

  function call(
    message: object,
    transfer: ArrayBuffer[],
  ): Promise<ProverOutbound> {
    const worker = port;
    if (!worker || disposed) {
      return Promise.reject(new Error("Groth16 prover worker is closed"));
    }
    const id = nextId;
    nextId += 1;
    return new Promise((resolve, reject) => {
      pending.set(id, { resolve, reject });
      try {
        worker.ref();
        worker.post({ ...message, id }, transfer);
      } catch (error) {
        pending.delete(id);
        if (pending.size === 0) {
          worker.unref();
        }
        reject(error instanceof Error ? error : new Error(String(error)));
      }
    });
  }

  return {
    exportBases(provingKey, r1cs) {
      return enqueue(async () => {
        await boot();
        const provingKeyBytes = copyBytes(provingKey);
        const r1csBytes = copyBytes(r1cs);
        const message = await call(
          { type: "export", provingKey: provingKeyBytes, r1cs: r1csBytes },
          [provingKeyBytes, r1csBytes],
        );
        if (message.type !== "bases") {
          throw new Error("Groth16 prover worker returned no bases");
        }
        return new Uint8Array(message.bases);
      });
    },
    prepareGroth16(graph, inputsJson) {
      return enqueue(async () => {
        await boot();
        const graphBytes = copyBytes(graph);
        const message = await call(
          { type: "prepare", graph: graphBytes, inputsJson },
          [graphBytes],
        );
        if (message.type !== "prepared") {
          throw new Error("Groth16 prover worker returned no scalars");
        }
        return {
          scalars: new Uint8Array(message.scalars),
          publicHex: message.publicHex,
        };
      });
    },
    terminate() {
      disposed = true;
      port?.close();
      port = null;
      failAll(new Error("Groth16 prover worker is closed"));
    },
  };
}

/** Node `Buffer#slice` shares memory, so transferring its buffer would detach the caller's artifact. */
function copyBytes(bytes: Uint8Array): ArrayBuffer {
  const copy = new Uint8Array(bytes.byteLength);
  copy.set(bytes);
  return copy.buffer;
}

function fingerprint(bytes: Uint8Array): string {
  let hash = 0x811c9dc5;
  for (let index = 0; index < bytes.length; index += 1) {
    hash ^= bytes[index] ?? 0;
    hash = Math.imul(hash, 0x01000193);
  }
  return `${bytes.length}:${hash >>> 0}`;
}
