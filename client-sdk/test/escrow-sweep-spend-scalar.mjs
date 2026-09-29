import assert from 'node:assert/strict';
import {
  BABYJUB_SUBGROUP_ORDER,
  withdrawObjectFromEscrowMerkleWitness,
  withdrawObjectFromMerkleWitness,
} from '../dist/index.mjs';

const HIGH_DERIVED =
  5643642338851415319256788894991006312577956583858606833408909008955561875305n;
const EXPECTED_REDUCED = HIGH_DERIVED % BABYJUB_SUBGROUP_ORDER;
assert.ok(HIGH_DERIVED >= BABYJUB_SUBGROUP_ORDER);
assert.equal(
  EXPECTED_REDUCED.toString(),
  '171581620891596513695187458676687540424328639541472315008477687058667129223',
);

const merkle = {
  withdrawnValue: '100',
  value: '100',
  nullifier: '1',
  secret: '2',
  withdrawnAsset: /** @type {[string, string]} */ (['3', '4']),
  stateRoot: '5',
  stateIndex: '0',
  stateSiblings: Array.from({ length: 20 }, () => '0'),
};
const ownerPub = { x: '0a'.padStart(64, '0'), y: '0b'.padStart(64, '0') };
const derivedHex = HIGH_DERIVED.toString(16).padStart(64, '0');

const withdraw = withdrawObjectFromEscrowMerkleWitness(
  merkle,
  ownerPub,
  '0',
  derivedHex,
  {
    nonce: '1',
    recipientHi: '7',
    recipientLo: '11',
  },
);

assert.equal(withdraw.privKeyScalar, EXPECTED_REDUCED.toString(10));
assert.equal(withdraw.escrowNonce, '1');
assert.deepEqual(withdraw.recipientStellar, ['7', '11']);
assert.ok(BigInt(withdraw.privKeyScalar) > 0n);
assert.ok(BigInt(withdraw.privKeyScalar) < BABYJUB_SUBGROUP_ORDER);

const registered = withdrawObjectFromMerkleWitness(
  merkle,
  ownerPub,
  '0',
  EXPECTED_REDUCED.toString(10),
);
assert.equal(registered.privKeyScalar, EXPECTED_REDUCED.toString(10));
assert.equal(registered.escrowNonce, '0');

console.log(
  `escrow-sweep-spend-scalar: ok reduced=${withdraw.privKeyScalar}`,
);
