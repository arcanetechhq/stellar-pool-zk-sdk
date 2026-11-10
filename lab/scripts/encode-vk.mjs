#!/usr/bin/env node
// Encode a snarkjs BN254 Groth16 verification key into the byte layout used
// by the privacy pool: uncompressed G1 (x||y), G2 (x.c1||x.c0||y.c1||y.c0),
// then a big-endian u32 count and the IC points. Affine z must be 1.
import { readFileSync } from "node:fs";

const P = 21888242871839275222246405745257275088696311157297823662689037894645226208583n;

function be32(value) {
  if (value < 0n || value >= P) {
    throw new Error(`field element out of range: ${value}`);
  }
  return Buffer.from(value.toString(16).padStart(64, "0"), "hex");
}

function g1(point) {
  const [x, y, z] = point.map(BigInt);
  if (z !== 1n) {
    throw new Error(`expected affine G1 z=1, got ${z}`);
  }
  return Buffer.concat([be32(x), be32(y)]);
}

function g2(point) {
  const [[x0, x1], [y0, y1], [z0, z1]] = point.map((limb) => limb.map(BigInt));
  if (z0 !== 1n || z1 !== 0n) {
    throw new Error(`expected affine G2 z=(1,0), got ${z0},${z1}`);
  }
  return Buffer.concat([be32(x1), be32(x0), be32(y1), be32(y0)]);
}

const vk = JSON.parse(readFileSync(process.argv[2], "utf8"));
const ic = vk.IC;
const count = Buffer.alloc(4);
count.writeUInt32BE(ic.length);
const bytes = Buffer.concat([
  g1(vk.vk_alpha_1),
  g2(vk.vk_beta_2),
  g2(vk.vk_gamma_2),
  g2(vk.vk_delta_2),
  count,
  ...ic.map(g1),
]);
process.stdout.write(bytes.toString("hex"));
console.error(
  `vk bytes=${bytes.length} ic=${ic.length} nPublic=${vk.nPublic}`,
);
