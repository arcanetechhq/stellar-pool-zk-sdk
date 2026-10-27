import { readFileSync } from "fs";
import { parentPort } from "worker_threads";
import { fileURLToPath } from "url";
import * as wasmBindings from "privacy-pool-client-sdk/pkg/client_sdk_wasm.js";
import { handleWorkerMessage, markWorkerStarted } from "./worker-session";
import type { WorkerInbound, WorkerOutbound } from "./protocol";

const port = parentPort;
if (!port) {
  throw new Error("Groth16 node worker needs a parent port");
}

const wasmPath = fileURLToPath(
  new URL("../pkg/client_sdk_wasm_bg.wasm", import.meta.url),
);
wasmBindings.initSync({ module: readFileSync(wasmPath) });
markWorkerStarted();

port.on("message", (message: WorkerInbound) => {
  if (message.type === "init") {
    port.postMessage({ type: "ready" } satisfies WorkerOutbound);
    return;
  }
  handleWorkerMessage(wasmBindings, message, (outbound, transfer) => {
    port.postMessage(outbound, transfer);
  });
});
