import { createRequire } from "node:module";

const { Keypair } = createRequire(
  new URL("../../client-sdk/package.json", import.meta.url),
)("@stellar/stellar-sdk");
import {
  APPLICATION_ID,
  DEPOSIT_NONCE,
  NOTE_VALUE,
  accountEd25519Hex,
  assetFields,
  auditFor,
  contractId,
  decimalToHex32,
  depositSdk,
  fetchState,
  fundAccount,
  invoke,
  merkleRootDecimal,
  readJson,
  openTrustline,
  recordTransaction,
  stellar,
  submitTransact,
  writeJson,
} from "./lib.mjs";
import {
  canonicalBabyJubScalarFromInteger,
  randomFrDecimal253,
} from "../../client-sdk/dist/index.node.mjs";

const deploy = readJson("deploy.json");

async function account(label) {
  const key = Keypair.random();
  await fundAccount(key.publicKey(), label);
  return { label, secret: key.secret(), address: key.publicKey() };
}

function deployAsset(code, issuer) {
  const output = stellar([
    "contract",
    "asset",
    "deploy",
    "--asset",
    `${code}:${issuer.address}`,
    "--source",
    issuer.secret,
    "--network",
    "testnet",
  ]).output;
  recordTransaction(`asset ${code}`, output);
  return { code, issuer: issuer.address, contract: contractId(output) };
}

function pay(code, issuer, destination) {
  const trust = stellar([
    "tx",
    "new",
    "change-trust",
    "--source",
    destination.secret,
    "--network",
    "testnet",
    "--line",
    `${code}:${issuer.address}`,
  ]).output;
  recordTransaction(`trust ${destination.label} ${code}`, trust);
  const payment = stellar([
    "tx",
    "new",
    "payment",
    "--source",
    issuer.secret,
    "--network",
    "testnet",
    "--destination",
    destination.address,
    "--asset",
    `${code}:${issuer.address}`,
    "--amount",
    "100000000",
  ]).output;
  recordTransaction(`pay ${destination.label} ${code}`, payment);
}

async function depositNote(party, assetContract) {
  const sdk = await depositSdk();
  const [assetHi, assetLo] = assetFields(assetContract);
  const spendScalar = canonicalBabyJubScalarFromInteger(
    BigInt(randomFrDecimal253()),
  ).toString(10);
  const spendHex = decimalToHex32(spendScalar);
  const owner = sdk.ecdhEphemeralPublicKeyFromScalarHex(spendHex);
  const ephemeralScalar = randomFrDecimal253();
  const ephemeralHex = decimalToHex32(ephemeralScalar);
  const generated = sdk.generateCoinForDepositWithOwnerPubHex(
    ephemeralHex,
    owner.x,
    owner.y,
    NOTE_VALUE,
    assetHi,
    assetLo,
    APPLICATION_ID,
  );
  const rootOutput = invoke(deploy.pool, "demo_user", "get_merkle_root", []);
  const deposit = {
    value: NOTE_VALUE,
    nullifier: generated.coin.nullifier,
    ephemeralKeyScalar: ephemeralScalar,
    recipientPublicKeys: [
      BigInt(`0x${owner.x}`).toString(10),
      BigInt(`0x${owner.y}`).toString(10),
    ],
    asset: [assetHi, assetLo],
    applicationId: APPLICATION_ID,
  };
  const proved = await sdk.proveTransaction(
    {
      stateRoot: merkleRootDecimal(rootOutput),
      withdrawAddressHi: "0",
      withdrawAddressLo: "0",
      privKeyScalar: randomFrDecimal253(),
    },
    {
      publicWithdrawnAssets: [["0", "0"]],
      publicDepositedAssets: [[assetHi, assetLo]],
      publicDeposits: [NOTE_VALUE],
      publicWithdrawals: ["0"],
    },
    ["dummy", "dummy"],
    [deposit, "dummy"],
    auditFor(sdk),
  );
  await submitTransact(
    `deposit ${party.label}`,
    party.secret,
    party.address,
    DEPOSIT_NONCE,
    {
      ...proved,
      passage: {
        owner: party.address,
        withdrawAddress: party.address,
        publicHex: proved.public_hex,
        ciphertextHex: proved.ciphertext_hex,
        publicDeposits: [NOTE_VALUE],
        publicWithdrawals: ["0"],
        publicDepositedAssets: [[assetHi, assetLo]],
        publicWithdrawnAssets: [["0", "0"]],
      },
    },
  );
  return {
    secret: {
      secret: party.secret,
      address: party.address,
      spendScalar,
      ephemeralScalar,
    },
    public: {
      address: party.address,
      ownerX: BigInt(`0x${owner.x}`).toString(10),
      ownerY: BigInt(`0x${owner.y}`).toString(10),
      asset: assetContract,
      assetHi,
      assetLo,
    },
    note: { ...generated.coin, application_id: APPLICATION_ID },
    ownerHex: accountEd25519Hex(party.address),
  };
}

const issuer = await account("issuer");
const sender = await account("sender");
const seller = await account("seller");
const senderAsset = deployAsset("SWPA", issuer);
const sellerAsset = deployAsset("SWPB", issuer);
pay(senderAsset.code, issuer, sender);
pay(sellerAsset.code, issuer, seller);
openTrustline(
  sender.secret,
  sellerAsset.code,
  issuer.address,
  "trust sender SWPB",
);
openTrustline(
  seller.secret,
  senderAsset.code,
  issuer.address,
  "trust seller SWPA",
);

const senderNotes = await depositNote(sender, senderAsset.contract);
const sellerNotes = await depositNote(seller, sellerAsset.contract);
writeJson("sender-secret.json", senderNotes.secret);
writeJson("seller-secret.json", sellerNotes.secret);
writeJson("sender-public.json", senderNotes.public);
writeJson("seller-public.json", sellerNotes.public);
writeJson("sender-note.json", senderNotes.note);
writeJson("seller-note.json", sellerNotes.note);
writeJson("pool-state.json", fetchState());
writeJson("assets.json", {
  issuer: issuer.address,
  sender: senderAsset,
  seller: sellerAsset,
});
console.log("accounts and notes are ready");
