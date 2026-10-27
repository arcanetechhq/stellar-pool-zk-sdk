import * as wasmBindings from "privacy-pool-client-sdk/pkg/client_sdk_wasm.js";

let wasmModule: typeof wasmBindings | null = null;

function alreadyInitialized(error: unknown): boolean {
  const message = error instanceof Error ? error.message : String(error);
  return /already initialized|more than once/i.test(message);
}

/** Node loader. The browser build replaces this file. */
export async function loadWasm(wasmBinary?: BufferSource): Promise<typeof wasmBindings> {
  if (wasmModule) {
    return wasmModule;
  }
  try {
    if (wasmBinary) {
      wasmBindings.initSync({ module: wasmBinary });
    } else {
      const path = await import("path");
      const { fileURLToPath } = await import("url");
      const fs = await import("fs");
      const dir = path.dirname(fileURLToPath(import.meta.url));
      const wasmPath = path.resolve(dir, "..", "pkg", "client_sdk_wasm_bg.wasm");
      wasmBindings.initSync({ module: fs.readFileSync(wasmPath) });
    }
  } catch (error) {
    if (!alreadyInitialized(error)) {
      throw error;
    }
  }
  wasmModule = wasmBindings;
  return wasmModule;
}
