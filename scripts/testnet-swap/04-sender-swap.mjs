import path from "node:path";
import {
  STATE,
  SWAP_NONCE,
  auditFor,
  decimalToHex32,
  fetchState,
  parsePublicSignalFields,
  readJson,
  submitTransact,
  swapSdk,
  writeJson,
} from "./lib.mjs";
const sender = readJson("sender-secret.json");
const senderNote = readJson("sender-note.json");
const offer = readJson("authorization.json");
const state = fetchState();
const witnessPath = path.join(STATE, "sender-witness.json");
const sdk = await swapSdk();
const proved = await sdk.proveSwap({
  authorization: offer.authorization,
  sellerNotes: offer.sellerNotes,
  senderNotes: [senderNote],
  senderSpendScalar: sender.spendScalar,
  state,
  audit: auditFor(sdk),
  witnessInputPath: witnessPath,
});
await submitTransact("swap", sender.secret, sender.address, SWAP_NONCE, {
  ...proved,
  passage: {
    owner: sender.address,
    withdrawAddress: sender.address,
    publicHex: proved.public_hex,
    ciphertextHex: proved.ciphertext_hex,
    publicDeposits: [],
    publicWithdrawals: [],
    publicDepositedAssets: [],
    publicWithdrawnAssets: [],
  },
});

const witness = readJson("sender-witness.json");
const fields = parsePublicSignalFields(proved.public_hex);
const ephemeralScalar = witness.depositedEphemeralKeyScalars[1];
const placeholder = sdk.generateCoinFromDepositEphemeralScalarHex(
  decimalToHex32(ephemeralScalar),
  witness.depositedValues[1],
  witness.depositedAssets[1][0],
  witness.depositedAssets[1][1],
  witness.outputApplicationIds[1],
);
writeJson("sender-output-note.json", {
  value: witness.depositedValues[1],
  nullifier: witness.depositedNullifiers[1],
  secret: placeholder.coin.secret,
  commitment: fields[7],
  asset_hi: witness.depositedAssets[1][0],
  asset_lo: witness.depositedAssets[1][1],
  application_id: witness.outputApplicationIds[1],
  ephemeralScalar,
});
console.log("swap accepted");
