import { createMsmPool, type PoolPort } from "./pool-core";
import type { Groth16Pool, WorkerOutbound } from "./protocol";

/** Browser MSM pool. Workers receive the same WASM bytes the page loaded. */
export function createGroth16Pool(wasmBinary?: BufferSource): Groth16Pool {
  const cores =
    typeof navigator === "undefined" ? 1 : navigator.hardwareConcurrency || 1;
  return createMsmPool({
    environment: "browser",
    cores,
    spawn(): PoolPort {
      const worker = new Worker(
        new URL("./groth16-worker.mjs", import.meta.url),
        { type: "module" },
      );
      return {
        post(message, transfer) {
          if (
            isInit(message) &&
            wasmBinary !== undefined &&
            !("wasm" in message)
          ) {
            const wasm = copyWasm(wasmBinary);
            worker.postMessage({ type: "init", wasm }, [wasm]);
            return;
          }
          worker.postMessage(message, transfer ?? []);
        },
        close() {
          worker.terminate();
        },
        listen(handler: (message: WorkerOutbound) => void) {
          worker.addEventListener("message", (event: MessageEvent<WorkerOutbound>) => {
            handler(event.data);
          });
        },
        fail(handler) {
          worker.addEventListener("error", () => {
            handler(new Error("Groth16 worker failed"));
          });
        },
        ref() {},
        unref() {},
      };
    },
  });
}

function isInit(message: object): message is { type: "init" } {
  return "type" in message && message.type === "init";
}

function copyWasm(binary: BufferSource): ArrayBuffer {
  if (binary instanceof ArrayBuffer) {
    return binary.slice(0);
  }
  const view = new Uint8Array(binary.buffer, binary.byteOffset, binary.byteLength);
  return view.slice().buffer;
}
