export type MsmCurve = "g1" | "g2";

export interface MsmJob {
  id: number;
  curve: MsmCurve;
  cacheKey: string;
  bases: Uint8Array;
  scalars: Uint8Array;
}

export interface MsmRequest {
  type: "msm";
  id: number;
  curve: MsmCurve;
  cacheKey: string;
  bases?: ArrayBuffer;
  scalars: ArrayBuffer;
}

export interface InitRequest {
  type: "init";
  wasm?: ArrayBuffer;
}

export interface ReadyMessage {
  type: "ready";
}

export interface ResultMessage {
  type: "result";
  id: number;
  point: ArrayBuffer;
}

export interface ErrorMessage {
  type: "error";
  id: number;
  message: string;
}

export type WorkerInbound = InitRequest | MsmRequest;
export type WorkerOutbound = ReadyMessage | ResultMessage | ErrorMessage;

export interface Groth16Pool {
  readonly environment: "browser" | "node";
  readonly cores: number;
  run(workerCount: number, jobs: MsmJob[]): Promise<Map<number, Uint8Array>>;
  terminate(): void;
}
