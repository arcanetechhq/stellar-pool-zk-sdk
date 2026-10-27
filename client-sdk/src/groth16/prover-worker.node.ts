import { readFileSync } from "fs";
import { parentPort } from "worker_threads";
import { fileURLToPath } from "url";
import * as wasmBindings from "privacy-pool-client-sdk/pkg/client_sdk_wasm.js";
import {
  handleProverMessage,
  markProverStarted,
  type ProverWasm,
} from "./prover-session";
import type { ProverInbound, ProverOutbound } from "./prover-protocol";

const port = parentPort;
if (!port) {
  throw new Error("Groth16 prover worker needs a parent port");
}

const wasmPath = fileURLToPath(
  new URL("../pkg/client_sdk_wasm_bg.wasm", import.meta.url),
);
wasmBindings.initSync({ module: readFileSync(wasmPath) });
markProverStarted();

port.on("message", (message: ProverInbound) => {
  if (message.type === "init") {
    port.postMessage({ type: "ready" } satisfies ProverOutbound);
    return;
  }
  handleProverMessage(
    wasmBindings as unknown as ProverWasm,
    message,
    (outbound, transfer) => {
      port.postMessage(outbound, transfer);
    },
  );
});
