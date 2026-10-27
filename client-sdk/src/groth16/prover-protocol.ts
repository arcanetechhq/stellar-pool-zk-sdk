import type { ErrorMessage, InitRequest, ReadyMessage } from "./protocol";

export interface ProverExportRequest {
  type: "export";
  id: number;
  provingKey: ArrayBuffer;
  r1cs: ArrayBuffer;
}

export interface ProverPrepareRequest {
  type: "prepare";
  id: number;
  graph: ArrayBuffer;
  inputsJson: string;
}

export type ProverInbound =
  | InitRequest
  | ProverExportRequest
  | ProverPrepareRequest;

export interface ProverBasesMessage {
  type: "bases";
  id: number;
  bases: ArrayBuffer;
}

export interface ProverPreparedMessage {
  type: "prepared";
  id: number;
  scalars: ArrayBuffer;
  publicHex: string;
}

export type ProverOutbound =
  | ReadyMessage
  | ProverBasesMessage
  | ProverPreparedMessage
  | ErrorMessage;

export interface ProverPort {
  post(message: object, transfer?: ArrayBuffer[]): void;
  close(): void;
  listen(handler: (message: ProverOutbound) => void): void;
  fail(handler: (error: Error) => void): void;
  ref(): void;
  unref(): void;
}

/** One worker owns the decompressed proving key for this page. */
export interface ProverHost {
  exportBases(provingKey: Uint8Array, r1cs: Uint8Array): Promise<Uint8Array>;
  prepareGroth16(
    graph: Uint8Array,
    inputsJson: string,
  ): Promise<{ scalars: Uint8Array; publicHex: string }>;
  terminate(): void;
}
