import { availableParallelism } from "os";
import { Worker } from "worker_threads";
import { createMsmPool, type PoolPort } from "./pool-core";
import type { Groth16Pool, WorkerOutbound } from "./protocol";

/** Node MSM pool. Rollup swaps this file for the browser pool. */
export function createGroth16Pool(_wasmBinary?: BufferSource): Groth16Pool {
  return createMsmPool({
    environment: "node",
    cores: availableParallelism(),
    spawn(): PoolPort {
      const worker = new Worker(
        new URL("./groth16-worker.node.mjs", import.meta.url),
        { type: "module" },
      );
      return {
        post(message, transfer) {
          worker.postMessage(message, transfer ?? []);
        },
        close() {
          void worker.terminate();
        },
        listen(handler: (message: WorkerOutbound) => void) {
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
  });
}
