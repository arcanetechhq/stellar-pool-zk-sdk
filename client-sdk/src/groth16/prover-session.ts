import type { ProverInbound, ProverOutbound } from "./prover-protocol";

export interface ProverWasm {
  initSync(module: { module: BufferSource }): void;
  default?: () => Promise<unknown>;
  exportMsmBases(provingKey: Uint8Array, r1cs: Uint8Array): Uint8Array;
  prepareGroth16(
    graph: Uint8Array,
    provingKey: Uint8Array,
    r1cs: Uint8Array,
    inputsJson: string,
  ): { scalars: Uint8Array; publicHex: string };
}

let started = false;
let provingKey: Uint8Array | null = null;
let r1cs: Uint8Array | null = null;

/** Node worker loads WASM itself, then accepts prover jobs. */
export function markProverStarted(): void {
  started = true;
}

export function handleProverMessage(
  wasm: ProverWasm,
  message: ProverInbound,
  post: (message: ProverOutbound, transfer?: ArrayBuffer[]) => void,
): void {
  if (message.type === "init") {
    startWasm(wasm, message.wasm);
    post({ type: "ready" });
    return;
  }
  if (!started) {
    post({
      type: "error",
      id: message.id,
      message: "Groth16 prover worker is not initialized",
    });
    return;
  }
  try {
    if (message.type === "export") {
      provingKey = new Uint8Array(message.provingKey);
      r1cs = new Uint8Array(message.r1cs);
      const bases = wasm.exportMsmBases(provingKey, r1cs).slice();
      post({ type: "bases", id: message.id, bases: bases.buffer }, [
        bases.buffer,
      ]);
      return;
    }
    if (!provingKey || !r1cs) {
      throw new Error("Groth16 bases are not exported");
    }
    const prepared = wasm.prepareGroth16(
      new Uint8Array(message.graph),
      provingKey,
      r1cs,
      message.inputsJson,
    );
    const scalars = prepared.scalars.slice();
    post(
      {
        type: "prepared",
        id: message.id,
        scalars: scalars.buffer,
        publicHex: prepared.publicHex,
      },
      [scalars.buffer],
    );
  } catch (error) {
    post({
      type: "error",
      id: message.id,
      message: error instanceof Error ? error.message : String(error),
    });
  }
}

function startWasm(wasm: ProverWasm, binary: ArrayBuffer | undefined): void {
  if (started) {
    return;
  }
  if (binary) {
    wasm.initSync({ module: binary });
  } else if (wasm.default) {
    throw new Error("Browser Groth16 prover worker needs the WASM bytes");
  } else {
    throw new Error("Groth16 prover worker has no WASM binary");
  }
  started = true;
}
