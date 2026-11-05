import {
  DEPOSIT_NONCE,
  auditFor,
  decimalToHex32,
  depositSdk,
  fetchState,
  openTrustline,
  readJson,
  submitTransact,
  withdrawAddressFields,
} from "./lib.mjs";

const sender = readJson("sender-secret.json");
const assets = readJson("assets.json");
openTrustline(
  sender.secret,
  assets.seller.code,
  assets.issuer,
  "trust sender SWPB",
);
const note = readJson("sender-output-note.json");
const sdk = await depositSdk();
const ephemeral = sdk.ecdhEphemeralPublicKeyFromScalarHex(
  decimalToHex32(note.ephemeralScalar),
);
const { hi: withdrawAddressHi, lo: withdrawAddressLo } = withdrawAddressFields(
  sender.address,
);
const proved = await sdk.proveWithdrawal(note, fetchState(), {
  withdrawAddressHi,
  withdrawAddressLo,
  privKeyScalar: sender.spendScalar,
  ephemeralXHex: ephemeral.x,
  ephemeralYHex: ephemeral.y,
  applicationId: note.application_id,
  audit: auditFor(sdk),
});
await submitTransact(
  "sender withdraw",
  sender.secret,
  sender.address,
  DEPOSIT_NONCE,
  {
    ...proved,
    passage: {
      owner: sender.address,
      withdrawAddress: sender.address,
      publicHex: proved.public_hex,
      ciphertextHex: proved.ciphertext_hex,
      publicDeposits: ["0"],
      publicWithdrawals: [note.value],
      publicDepositedAssets: [["0", "0"]],
      publicWithdrawnAssets: [[note.asset_hi, note.asset_lo]],
    },
  },
);
console.log("sender withdrawal accepted");
