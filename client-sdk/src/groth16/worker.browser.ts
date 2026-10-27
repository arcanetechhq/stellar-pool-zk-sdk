import * as wasmBindings from "privacy-pool-client-sdk/pkg/client_sdk_wasm.js";
import { handleWorkerMessage } from "./worker-session";
import type { WorkerInbound, WorkerOutbound } from "./protocol";

const scope = self as unknown as {
  onmessage: ((event: MessageEvent<WorkerInbound>) => void) | null;
  postMessage(message: WorkerOutbound, transfer?: ArrayBuffer[]): void;
};

scope.onmessage = (event) => {
  handleWorkerMessage(wasmBindings, event.data, (message, transfer) => {
    scope.postMessage(message, transfer);
  });
};
