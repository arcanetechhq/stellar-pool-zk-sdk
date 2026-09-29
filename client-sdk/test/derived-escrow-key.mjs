import assert from 'node:assert/strict';
import {
  BABYJUB_SUBGROUP_ORDER,
  sampleDerivedEscrowKey,
} from '../dist/index.mjs';

const MIN_RANDOM_NONCE = 1n << 127n;

const first = await sampleDerivedEscrowKey(1n, 2n);
const second = await sampleDerivedEscrowKey(1n, 2n);
assert.notEqual(first.nonce, second.nonce);
assert.ok(first.nonce >= MIN_RANDOM_NONCE);
assert.ok(second.nonce >= MIN_RANDOM_NONCE);
const firstScalar = BigInt(`0x${first.key.scalarHex}`);
const secondScalar = BigInt(`0x${second.key.scalarHex}`);
assert.ok(firstScalar > 0n && firstScalar < BABYJUB_SUBGROUP_ORDER);
assert.ok(secondScalar > 0n && secondScalar < BABYJUB_SUBGROUP_ORDER);

const supplied = 4n;
const honoured = await sampleDerivedEscrowKey(1n, 2n, supplied);
assert.equal(honoured.nonce, supplied);
const honouredScalar = BigInt(`0x${honoured.key.scalarHex}`);
assert.ok(honouredScalar > 0n && honouredScalar < BABYJUB_SUBGROUP_ORDER);

assert.equal(first.key.scalarHex.length, 64);
assert.equal(first.key.pointXHex.length, 64);
assert.equal(first.key.pointYHex.length, 64);
console.log(
  `derived-escrow-key: ok random=${first.nonce.toString()} supplied=${honoured.nonce.toString()}`,
);
