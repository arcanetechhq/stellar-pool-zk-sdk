import type { ProverHost } from "./prover-protocol";
import type { Groth16Pool, MsmCurve, MsmJob } from "./protocol";
import { workerCount } from "./schedule";

const HEADER_BYTES = 12;
const SCALAR_BYTES = 32;
const G1_BYTES = 64;
const G2_BYTES = 128;
const CONSTANTS_BYTES = G1_BYTES * 5 + G2_BYTES * 3;

export interface Groth16Wasm {
  proveGroth16(
    graph: Uint8Array,
    provingKey: Uint8Array,
    r1cs: Uint8Array,
    inputsJson: string,
  ): { proof_hex: string; public_hex: string };
  prepareGroth16(
    graph: Uint8Array,
    provingKey: Uint8Array,
    r1cs: Uint8Array,
    inputsJson: string,
  ): { scalars: Uint8Array; publicHex: string };
  exportMsmBases(provingKey: Uint8Array, r1cs: Uint8Array): Uint8Array;
  sumG1(points: Uint8Array): Uint8Array;
  sumG2(points: Uint8Array): Uint8Array;
  finishGroth16(
    constants: Uint8Array,
    a: Uint8Array,
    bG1: Uint8Array,
    bG2: Uint8Array,
    h: Uint8Array,
    l: Uint8Array,
    r: Uint8Array,
    s: Uint8Array,
  ): Uint8Array;
}

export interface ParallelProveInput {
  wasm: Groth16Wasm;
  pool: Groth16Pool;
  host: ProverHost | null;
  graph: Uint8Array;
  provingKey: Uint8Array;
  r1cs: Uint8Array;
  inputsJson: string;
  cachedBases: Uint8Array | null;
}

export interface ParallelProveResult {
  proof_hex: string;
  public_hex: string;
  bases: Uint8Array | null;
}

/** One Groth16 proof. A single worker falls back to the monolithic prover. */
export async function proveGroth16Parallel(
  input: ParallelProveInput,
): Promise<ParallelProveResult> {
  const monolithic = () =>
    input.wasm.proveGroth16(
      input.graph,
      input.provingKey,
      input.r1cs,
      input.inputsJson,
    );
  try {
    const bases =
      input.cachedBases ??
      (input.host
        ? await input.host.exportBases(input.provingKey, input.r1cs)
        : input.wasm.exportMsmBases(input.provingKey, input.r1cs));
    const prepared = input.host
      ? await input.host.prepareGroth16(input.graph, input.inputsJson)
      : input.wasm.prepareGroth16(
          input.graph,
          input.provingKey,
          input.r1cs,
          input.inputsJson,
        );
    const lengths = readLengths(prepared.scalars);
    const count = workerCount({
      cores: input.pool.cores,
      environment: input.pool.environment,
      scalarCount: lengths.assignmentLen,
    });
    if (count < 2) {
      const proof = monolithic();
      return { ...proof, bases };
    }
    const jobs = buildJobs(prepared.scalars, bases, count);
    const points = await input.pool.run(count, jobs);
    const proof = input.wasm.finishGroth16(
      constantsOf(bases),
      summed(input.wasm, points, jobs, "a", "g1"),
      summed(input.wasm, points, jobs, "bG1", "g1"),
      summed(input.wasm, points, jobs, "bG2", "g2"),
      summed(input.wasm, points, jobs, "h", "g1"),
      summed(input.wasm, points, jobs, "l", "g1"),
      scalarAt(prepared.scalars, lengths.assignmentLen + lengths.hLen),
      scalarAt(prepared.scalars, lengths.assignmentLen + lengths.hLen + 1),
    );
    return {
      proof_hex: toHex(proof),
      public_hex: prepared.publicHex,
      bases,
    };
  } catch (error) {
    console.warn("Parallel Groth16 fell back to one thread", error);
    const proof = monolithic();
    return { ...proof, bases: input.cachedBases };
  }
}

function buildJobs(
  scalars: Uint8Array,
  bases: Uint8Array,
  parts: number,
): MsmJob[] {
  const lengths = readLengths(scalars);
  const baseLengths = readLengths(bases);
  if (
    lengths.assignmentLen !== baseLengths.assignmentLen ||
    lengths.hLen !== baseLengths.hLen ||
    lengths.auxLen !== baseLengths.auxLen
  ) {
    throw new Error("Groth16 scalar and base lengths differ");
  }
  const regions = baseRegions(bases, baseLengths);
  const assignmentScalars = sliceScalars(scalars, 0, lengths.assignmentLen);
  const hScalars = sliceScalars(scalars, lengths.assignmentLen, lengths.hLen);
  const auxScalars = sliceScalars(
    scalars,
    lengths.assignmentLen - lengths.auxLen,
    lengths.auxLen,
  );
  const series: Array<{
    name: string;
    curve: MsmCurve;
    bases: Uint8Array;
    scalars: Uint8Array;
    width: number;
  }> = [
    {
      name: "bG2",
      curve: "g2",
      bases: regions.bG2,
      scalars: assignmentScalars,
      width: G2_BYTES,
    },
    {
      name: "a",
      curve: "g1",
      bases: regions.a,
      scalars: assignmentScalars,
      width: G1_BYTES,
    },
    {
      name: "bG1",
      curve: "g1",
      bases: regions.bG1,
      scalars: assignmentScalars,
      width: G1_BYTES,
    },
    {
      name: "h",
      curve: "g1",
      bases: regions.h,
      scalars: hScalars,
      width: G1_BYTES,
    },
    {
      name: "l",
      curve: "g1",
      bases: regions.l,
      scalars: auxScalars,
      width: G1_BYTES,
    },
  ];
  const jobs: MsmJob[] = [];
  let id = 1;
  for (const entry of series) {
    if (entry.scalars.byteLength === 0) {
      continue;
    }
    for (const range of ranges(
      entry.scalars.byteLength / SCALAR_BYTES,
      parts,
    )) {
      jobs.push({
        id,
        curve: entry.curve,
        cacheKey: `${entry.name}:${range.start}:${range.end}`,
        bases: sliceElements(
          entry.bases,
          range.start,
          range.end - range.start,
          entry.width,
        ),
        scalars: sliceElements(
          entry.scalars,
          range.start,
          range.end - range.start,
          SCALAR_BYTES,
        ),
      });
      id += 1;
    }
  }
  return jobs;
}

function summed(
  wasm: Groth16Wasm,
  points: Map<number, Uint8Array>,
  jobs: MsmJob[],
  name: string,
  curve: MsmCurve,
): Uint8Array {
  const grouped = jobs.filter((job) => job.cacheKey.startsWith(`${name}:`));
  const width = curve === "g1" ? G1_BYTES : G2_BYTES;
  if (grouped.length === 0) {
    return new Uint8Array(width);
  }
  const packed = new Uint8Array(grouped.length * width);
  grouped.forEach((job, index) => {
    const point = points.get(job.id);
    if (!point || point.byteLength !== width) {
      throw new Error(`Missing ${name} MSM result ${job.id}`);
    }
    packed.set(point, index * width);
  });
  if (grouped.length === 1) {
    return packed;
  }
  return curve === "g1" ? wasm.sumG1(packed) : wasm.sumG2(packed);
}

function ranges(
  count: number,
  parts: number,
): Array<{ start: number; end: number }> {
  if (count === 0) {
    return [];
  }
  const width = Math.max(1, Math.ceil(count / parts));
  const out: Array<{ start: number; end: number }> = [];
  for (let start = 0; start < count; start += width) {
    out.push({ start, end: Math.min(count, start + width) });
  }
  return out;
}

function readLengths(bytes: Uint8Array): {
  assignmentLen: number;
  hLen: number;
  auxLen: number;
} {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  return {
    assignmentLen: view.getUint32(0, true),
    hLen: view.getUint32(4, true),
    auxLen: view.getUint32(8, true),
  };
}

function baseRegions(
  bases: Uint8Array,
  lengths: { assignmentLen: number; hLen: number; auxLen: number },
): {
  a: Uint8Array;
  bG1: Uint8Array;
  bG2: Uint8Array;
  h: Uint8Array;
  l: Uint8Array;
} {
  let offset = HEADER_BYTES;
  const take = (bytes: number): Uint8Array => {
    const slice = bases.subarray(offset, offset + bytes);
    offset += bytes;
    return slice;
  };
  return {
    a: take(lengths.assignmentLen * G1_BYTES),
    bG1: take(lengths.assignmentLen * G1_BYTES),
    bG2: take(lengths.assignmentLen * G2_BYTES),
    h: take(lengths.hLen * G1_BYTES),
    l: take(lengths.auxLen * G1_BYTES),
  };
}

function constantsOf(bases: Uint8Array): Uint8Array {
  return bases.subarray(bases.byteLength - CONSTANTS_BYTES);
}

function sliceScalars(
  bytes: Uint8Array,
  index: number,
  count: number,
): Uint8Array {
  const start = HEADER_BYTES + index * SCALAR_BYTES;
  return bytes.subarray(start, start + count * SCALAR_BYTES);
}

function scalarAt(bytes: Uint8Array, index: number): Uint8Array {
  return sliceScalars(bytes, index, 1);
}

function sliceElements(
  bytes: Uint8Array,
  start: number,
  count: number,
  width: number,
): Uint8Array {
  return bytes.subarray(start * width, (start + count) * width);
}

function toHex(bytes: Uint8Array): string {
  let hex = "";
  for (const byte of bytes) {
    hex += byte.toString(16).padStart(2, "0");
  }
  return hex;
}
