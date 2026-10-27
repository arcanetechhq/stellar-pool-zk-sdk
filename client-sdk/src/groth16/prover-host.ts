import { Worker } from "worker_threads";
import { sharedProver } from "./prover-client";
import type { ProverOutbound, ProverPort } from "./prover-protocol";

/** Node prover worker. Rollup swaps this file for the browser host. */
export function openProver(
  _wasmBinary: BufferSource | undefined,
  provingKey: Uint8Array,
  r1cs: Uint8Array,
): ReturnType<typeof sharedProver> {
  return sharedProver(
    (): ProverPort => {
      const worker = new Worker(
        new URL("./prover-worker.node.mjs", import.meta.url),
        { type: "module" },
      );
      return {
        post(message, transfer) {
          worker.postMessage(message, transfer ?? []);
        },
        close() {
          void worker.terminate();
        },
        listen(handler: (message: ProverOutbound) => void) {
          worker.on("message", handler);
        },
        fail(handler) {
          worker.on("error", handler);
        },
        ref() {
          worker.ref();
        },
        unref() {
          worker.unref();
        },
      };
    },
    provingKey,
    r1cs,
  );
}
