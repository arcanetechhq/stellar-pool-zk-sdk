import * as wasmBindings from "privacy-pool-client-sdk/pkg/client_sdk_wasm.js";
import { handleProverMessage, type ProverWasm } from "./prover-session";
import type { ProverInbound, ProverOutbound } from "./prover-protocol";

const scope = self as unknown as {
  onmessage: ((event: MessageEvent<ProverInbound>) => void) | null;
  postMessage(message: ProverOutbound, transfer?: ArrayBuffer[]): void;
};

scope.onmessage = (event) => {
  handleProverMessage(
    wasmBindings as unknown as ProverWasm,
    event.data,
    (message, transfer) => {
      scope.postMessage(message, transfer);
    },
  );
};
