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

const seller = readJson("seller-secret.json");
const assets = readJson("assets.json");
openTrustline(
  seller.secret,
  assets.sender.code,
  assets.issuer,
  "trust seller SWPA",
);
const offer = readJson("authorization.json");
const opening = offer.authorization.noteOpenings[0];
const note = {
  value: opening.value,
  nullifier: opening.nullifier,
  secret: opening.secret,
  commitment: opening.commitment,
  asset_hi: opening.asset_hi,
  asset_lo: opening.asset_lo,
  application_id: opening.application_id,
};
const sdk = await depositSdk();
const ephemeral = sdk.ecdhEphemeralPublicKeyFromScalarHex(
  decimalToHex32(opening.ephemeral_key_scalar),
);
const { hi: withdrawAddressHi, lo: withdrawAddressLo } = withdrawAddressFields(
  seller.address,
);
const proved = await sdk.proveWithdrawal(note, fetchState(), {
  withdrawAddressHi,
  withdrawAddressLo,
  privKeyScalar: seller.spendScalar,
  ephemeralXHex: ephemeral.x,
  ephemeralYHex: ephemeral.y,
  applicationId: note.application_id,
  audit: auditFor(sdk),
});
await submitTransact(
  "seller withdraw",
  seller.secret,
  seller.address,
  DEPOSIT_NONCE,
  {
    ...proved,
    passage: {
      owner: seller.address,
      withdrawAddress: seller.address,
      publicHex: proved.public_hex,
      ciphertextHex: proved.ciphertext_hex,
      publicDeposits: ["0"],
      publicWithdrawals: [note.value],
      publicDepositedAssets: [["0", "0"]],
      publicWithdrawnAssets: [[note.asset_hi, note.asset_lo]],
    },
  },
);
console.log("seller withdrawal accepted");
