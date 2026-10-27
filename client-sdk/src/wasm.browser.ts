import * as wasmBindings from "privacy-pool-client-sdk/pkg/client_sdk_wasm.js";

let wasmModule: typeof wasmBindings | null = null;

function alreadyInitialized(error: unknown): boolean {
  const message = error instanceof Error ? error.message : String(error);
  return /already initialized|more than once/i.test(message);
}

/** Browser loader. Callers pass the WASM bytes; there is no filesystem. */
export async function loadWasm(
  wasmBinary?: BufferSource,
): Promise<typeof wasmBindings> {
  if (wasmModule) {
    return wasmModule;
  }
  try {
    if (wasmBinary) {
      wasmBindings.initSync({ module: wasmBinary });
    } else {
      await wasmBindings.default();
    }
  } catch (error) {
    if (!alreadyInitialized(error)) {
      throw error;
    }
  }
  wasmModule = wasmBindings;
  return wasmModule;
}
