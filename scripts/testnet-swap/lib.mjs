import { createRequire } from "node:module";
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const require = createRequire(
  new URL("../../client-sdk/package.json", import.meta.url),
);
const { Keypair, StrKey } = require("@stellar/stellar-sdk");
import {
  PrivacyPoolSDK,
  SWAP_ZK_NONCE,
  buildKytPassageAuthorization,
  derivePassageFromTransactContext,
  ed25519PubkeyPayloadHexToWithdrawFrDecimals,
  layoutForKnownNonce,
  resolveTransactionAuditParams,
  stellarContractAddressToAssetFrDecimals,
} from "../../client-sdk/dist/index.node.mjs";

const here = path.dirname(fileURLToPath(import.meta.url));

export const ROOT = path.resolve(here, "../..");
export const STATE = path.join(here, "state");
export const POOL_SRC =
  "/Users/serafimgerasimov/projects/arcane/stellar-pool-zk-worktrees/swap-deploy";
export const ARTIFACTS = path.join(ROOT, "artifacts");
export const SWAP_ARTIFACTS = path.join(
  ROOT,
  "soroban-privacy-pools/circuits/output",
);
export const NETWORK = "testnet";
export const RPC_URL =
  process.env.STELLAR_RPC_URL?.trim() || "https://soroban-testnet.stellar.org";
export const APPLICATION_ID = "101";
export const NOTE_VALUE = "100000000";
export const DEPOSIT_NONCE = 7n;
export const SWAP_NONCE = 8n;
export const BN254 =
  21888242871839275222246405745257275088548364400416034343698204186575808495617n;

export function ensureState() {
  fs.mkdirSync(STATE, { recursive: true });
}

export function readJson(name) {
  return JSON.parse(fs.readFileSync(path.join(STATE, name), "utf8"));
}

export function writeJson(name, value) {
  ensureState();
  const file = path.join(STATE, name);
  fs.writeFileSync(file, `${JSON.stringify(value, null, 2)}\n`);
  return file;
}

export function stellar(args, { allowFailure = false } = {}) {
  const result = spawnSync("stellar", args, {
    encoding: "utf8",
    maxBuffer: 64 * 1024 * 1024,
  });
  const output = `${result.stdout ?? ""}${result.stderr ?? ""}`;
  if (result.status !== 0 && !allowFailure) {
    const redacted = `${args.join(" ")}\n${output}`.replace(
      /S[A-Z0-9]{55}/g,
      "S…",
    );
    throw new Error(`stellar ${redacted}`);
  }
  return { status: result.status ?? 1, output };
}

export function contractId(output) {
  const ids = [...output.matchAll(/C[A-Z0-9]{55}/g)].map((match) => match[0]);
  if (ids.length === 0) {
    throw new Error(`no contract id in:\n${output}`);
  }
  return ids[ids.length - 1];
}

export function transactionId(output) {
  const labeled = output.match(/transaction (?:hash|id):\s*([0-9a-fA-F]{64})/i);
  if (labeled) return labeled[1].toLowerCase();
  const url = output.match(/\/tx\/([0-9a-fA-F]{64})/);
  if (url) return url[1].toLowerCase();
  const signing = output.match(/Signing transaction:?\s*([0-9a-fA-F]{64})/i);
  if (signing) return signing[1].toLowerCase();
  return null;
}

export function recordTransaction(label, output) {
  const id = transactionId(output);
  if (!id) {
    const log = path.join(STATE, "logs", `${label}.txt`);
    fs.mkdirSync(path.dirname(log), { recursive: true });
    fs.writeFileSync(log, output);
    throw new Error(`no transaction id for ${label}; output saved to ${log}`);
  }
  const file = path.join(STATE, "transactions.json");
  const current = fs.existsSync(file)
    ? JSON.parse(fs.readFileSync(file, "utf8"))
    : [];
  current.push({ label, id });
  fs.writeFileSync(file, `${JSON.stringify(current, null, 2)}\n`);
  console.log(`${label}: ${id}`);
  return id;
}

export function invoke(contract, source, method, args) {
  const output = stellar([
    "contract",
    "invoke",
    "--id",
    contract,
    "--source",
    source,
    "--network",
    NETWORK,
    "--",
    method,
    ...args,
  ]).output;
  return output;
}

export function invokeAndRecord(label, contract, source, method, args) {
  const output = invoke(contract, source, method, args);
  recordTransaction(label, output);
  return output;
}

export function fieldFromBytes(hex) {
  const normalized = hex.replace(/^0x/i, "").toLowerCase();
  return (BigInt(`0x${normalized}`) % BN254).toString(10);
}

export function decimalToHex32(decimal) {
  return BigInt(decimal).toString(16).padStart(64, "0");
}

export function parsePublicSignalFields(publicHex) {
  const bytes = Buffer.from(publicHex.trim().replace(/^0x/i, ""), "hex");
  let offset = 0;
  if (bytes.length >= 4) {
    const declared = bytes.readUInt32BE(0);
    if (bytes.length === 4 + declared * 32) offset = 4;
  }
  const body = bytes.subarray(offset);
  if (body.length % 32 !== 0) {
    throw new Error("public signals are not 32-byte fields");
  }
  const fields = [];
  for (let index = 0; index < body.length; index += 32) {
    fields.push(
      fieldFromBytes(body.subarray(index, index + 32).toString("hex")),
    );
  }
  return fields;
}

export function accountEd25519Hex(address) {
  return Buffer.from(StrKey.decodeEd25519PublicKey(address)).toString("hex");
}

export async function fundAccount(address, label) {
  const response = await fetch(`https://friendbot.stellar.org?addr=${address}`);
  const body = await response.text();
  if (!response.ok) {
    throw new Error(`friendbot ${label}: ${body}`);
  }
  const hash = transactionId(body) ?? JSON.parse(body).hash;
  if (hash) recordTransaction(`fund ${label}`, `transaction hash: ${hash}`);
  return hash;
}

export async function latestLedger() {
  const response = await fetch(RPC_URL, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ jsonrpc: "2.0", id: 1, method: "getLatestLedger" }),
  });
  const body = await response.json();
  const sequence = body.result?.sequence ?? body.result?.id;
  if (!sequence)
    throw new Error(`getLatestLedger failed: ${JSON.stringify(body)}`);
  return Number(sequence);
}

export function assetFields(contractId) {
  return stellarContractAddressToAssetFrDecimals(contractId);
}

export function openTrustline(sourceSecret, code, issuerAddress, label) {
  const output = stellar([
    "tx",
    "new",
    "change-trust",
    "--source",
    sourceSecret,
    "--network",
    "testnet",
    "--line",
    `${code}:${issuerAddress}`,
  ]).output;
  recordTransaction(label, output);
  return output;
}

export async function depositSdk() {
  return PrivacyPoolSDK.init({
    zkConfigNonce: DEPOSIT_NONCE,
    zkArtifactBaseUrl: ARTIFACTS,
  });
}

export async function swapSdk() {
  const graph = fs.readFileSync(path.join(SWAP_ARTIFACTS, "tx_swap.graph.bin"));
  const r1cs = fs.readFileSync(path.join(SWAP_ARTIFACTS, "tx_swap.r1cs.gz"));
  const provingKey = fs.readFileSync(
    path.join(SWAP_ARTIFACTS, "tx_swap_proving_key.bin"),
  );
  return PrivacyPoolSDK.init({
    zkConfigNonce: SWAP_ZK_NONCE,
    zkCircuits: {
      [SWAP_ZK_NONCE.toString()]: {
        nIns: 6,
        nOuts: 6,
        publicNInputs: 0,
        publicNOutputs: 0,
        circuitGraph: graph,
        r1cs,
        provingKey,
      },
    },
  });
}

export function withdrawAddressFields(address) {
  const hex = accountEd25519Hex(address);
  return ed25519PubkeyPayloadHexToWithdrawFrDecimals(hex);
}

export async function signPassage(parameters) {
  const deploy = readJson("deploy.json");
  const nonce = BigInt(parameters.nonce);
  const layout = layoutForKnownNonce(nonce);
  const derivation = derivePassageFromTransactContext({
    owner: parameters.owner,
    nonce,
    publicSignalsBytes: parameters.publicHex,
    ciphertextBytes: parameters.ciphertextHex,
    layout: {
      publicNInputs: layout.publicNInputs,
      publicNOutputs: layout.publicNOutputs,
    },
    publicLegContext: {
      owner: parameters.owner,
      withdrawAddress: parameters.withdrawAddress,
      publicDeposits: parameters.publicDeposits,
      publicWithdrawals: parameters.publicWithdrawals,
      publicDepositedAssets: parameters.publicDepositedAssets,
      publicWithdrawnAssets: parameters.publicWithdrawnAssets,
    },
  });
  const authorization = buildKytPassageAuthorization({
    poolContract: deploy.pool,
    kytRegistry: deploy.kytRegistry,
    passageId: derivation.passageId,
    expiresAtLedger: parameters.expiresAtLedger,
  });
  const signature = Keypair.fromSecret(deploy.kytSignerSecret)
    .sign(Buffer.from(authorization.authorizationHash, "hex"))
    .toString("hex");
  return JSON.stringify({
    expiration_ledger: parameters.expiresAtLedger,
    signature,
  });
}

export async function submitTransact(
  label,
  sourceSecret,
  fromAddress,
  nonce,
  proof,
) {
  const deploy = readJson("deploy.json");
  const expiresAtLedger = (await latestLedger()) + 1000;
  const kyt = await signPassage({ ...proof.passage, nonce, expiresAtLedger });
  const output = invoke(deploy.pool, sourceSecret, "transact", [
    "--from",
    fromAddress,
    "--nonce",
    nonce.toString(),
    "--proof_bytes",
    proof.proof_hex,
    "--pub_signals_bytes",
    proof.public_hex,
    "--ciphertext_bytes",
    proof.ciphertext_hex,
    "--kyt_authorization",
    kyt,
  ]);
  recordTransaction(label, output);
  return output;
}

export function merkleRootDecimal(output) {
  const match = output.match(/[0-9a-fA-F]{64}/);
  if (!match) throw new Error(`no merkle root in:\n${output}`);
  return fieldFromBytes(match[0]);
}

export function fetchState() {
  const deploy = readJson("deploy.json");
  const output = invoke(deploy.pool, "demo_user", "get_commitments", []);
  const line = output
    .split(/\r?\n/)
    .map((item) => item.trim())
    .reverse()
    .find((item) => item.startsWith("["));
  if (!line) throw new Error(`commitments missing:\n${output}`);
  const commitments = JSON.parse(line).map((hex) =>
    fieldFromBytes(String(hex)),
  );
  return { commitments };
}

export function auditFor(sdk) {
  return resolveTransactionAuditParams(
    APPLICATION_ID,
    undefined,
    sdk.getLayout().nAuditSlots,
  );
}
