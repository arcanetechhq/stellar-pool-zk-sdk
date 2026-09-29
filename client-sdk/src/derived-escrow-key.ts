import { loadWasm } from './wasm';
import {
  BABYJUB_SUBGROUP_ORDER,
} from './domain-separators';

export type DerivedEscrowKey = {
  scalarHex: string;
  pointXHex: string;
  pointYHex: string;
};

const SCALAR_HEX_LENGTH = 64;
const OUT_OF_RANGE_MESSAGE =
  'derived escrow hash is outside BabyJub subgroup; retry nonce';
const RANDOM_NONCE_BYTES = 16;
const MIN_RANDOM_NONCE = 1n << 127n;

function hexFromCanonicalScalar(value: bigint): string {
  return value.toString(16).padStart(SCALAR_HEX_LENGTH, '0');
}

function assertCanonicalBabyJubScalar(scalar: bigint): void {
  if (!(scalar > 0n && scalar < BABYJUB_SUBGROUP_ORDER)) {
    throw new Error(OUT_OF_RANGE_MESSAGE);
  }
}

function sampleRandomEscrowNonce(): bigint {
  const g = globalThis.crypto;
  if (!g?.getRandomValues) {
    throw new Error('derived-escrow-key: crypto.getRandomValues is required');
  }
  const bytes = new Uint8Array(RANDOM_NONCE_BYTES);
  g.getRandomValues(bytes);
  let value = 0n;
  for (const byte of bytes) {
    value = (value << 8n) | BigInt(byte);
  }
  return value | MIN_RANDOM_NONCE;
}

export async function derivedEscrowKey(
  nonce: bigint,
  recipientHi: bigint,
  recipientLo: bigint,
): Promise<DerivedEscrowKey> {
  const wasm = await loadWasm();
  const out = wasm.derivedEscrowKey(
    nonce.toString(),
    recipientHi.toString(),
    recipientLo.toString(),
  ) as DerivedEscrowKey;
  const scalar = BigInt(`0x${out.scalarHex}`);
  assertCanonicalBabyJubScalar(scalar);
  return {
    scalarHex: hexFromCanonicalScalar(scalar),
    pointXHex: out.pointXHex,
    pointYHex: out.pointYHex,
  };
}

export async function sampleDerivedEscrowKey(
  recipientHi: bigint,
  recipientLo: bigint,
  startNonce?: bigint,
  window = 1024n,
): Promise<{ nonce: bigint; key: DerivedEscrowKey }> {
  const baseNonce = startNonce ?? sampleRandomEscrowNonce();
  for (let offset = 0n; offset < window; offset += 1n) {
    const nonce = baseNonce + offset;
    try {
      const key = await derivedEscrowKey(nonce, recipientHi, recipientLo);
      return { nonce, key };
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      if (!message.includes('outside BabyJub subgroup')) {
        throw error;
      }
    }
  }
  throw new Error(OUT_OF_RANGE_MESSAGE);
}
