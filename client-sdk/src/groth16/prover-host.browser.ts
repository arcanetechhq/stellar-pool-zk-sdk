import { sharedProver } from "./prover-client";
import type { ProverOutbound, ProverPort } from "./prover-protocol";

/** Browser prover worker. The page passes the same WASM bytes it loaded. */
export function openProver(
  wasmBinary: BufferSource | undefined,
  provingKey: Uint8Array,
  r1cs: Uint8Array,
): ReturnType<typeof sharedProver> {
  return sharedProver(
    (): ProverPort => {
      const worker = new Worker(
        new URL("./prover-worker.mjs", import.meta.url),
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
        listen(handler: (message: ProverOutbound) => void) {
          worker.addEventListener(
            "message",
            (event: MessageEvent<ProverOutbound>) => {
              handler(event.data);
            },
          );
        },
        fail(handler) {
          worker.addEventListener("error", () => {
            handler(new Error("Groth16 prover worker failed"));
          });
        },
        ref() {},
        unref() {},
      };
    },
    provingKey,
    r1cs,
  );
}

function isInit(message: object): message is { type: "init" } {
  return "type" in message && message.type === "init";
}

function copyWasm(binary: BufferSource): ArrayBuffer {
  if (binary instanceof ArrayBuffer) {
    return binary.slice(0);
  }
  const view = new Uint8Array(
    binary.buffer,
    binary.byteOffset,
    binary.byteLength,
  );
  return view.slice().buffer;
}
