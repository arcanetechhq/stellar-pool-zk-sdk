import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { createRequire } from "node:module";

const { Keypair } = createRequire(
  new URL("../../client-sdk/package.json", import.meta.url),
)("@stellar/stellar-sdk");
import {
  ARTIFACTS,
  POOL_SRC,
  ROOT,
  SWAP_ARTIFACTS,
  STATE,
  contractId,
  ensureState,
  recordTransaction,
  stellar,
  writeJson,
} from "./lib.mjs";

const wasmDir = path.join(POOL_SRC, "target/wasm32v1-none/release");

function optimize(name) {
  const input = path.join(wasmDir, `${name}.wasm`);
  const output = path.join(wasmDir, `${name}.optimized.wasm`);
  if (!fs.existsSync(input)) {
    throw new Error(
      `missing ${input}. Build the swap worktree contracts first.`,
    );
  }
  stellar(["contract", "optimize", "--wasm", input, "--wasm-out", output]);
  return output;
}

function deploy(wasm, source, initArgs) {
  const output = stellar([
    "contract",
    "deploy",
    "--wasm",
    wasm,
    "--source",
    source,
    "--network",
    "testnet",
    "--",
    ...initArgs,
  ]).output;
  recordTransaction(`deploy ${path.basename(wasm, ".optimized.wasm")}`, output);
  return contractId(output);
}

function registerConfig(args) {
  const result = spawnSync(
    path.join(POOL_SRC, "scripts/add_zk_config.sh"),
    args,
    {
      encoding: "utf8",
      maxBuffer: 64 * 1024 * 1024,
    },
  );
  const output = `${result.stdout ?? ""}${result.stderr ?? ""}`;
  if (result.status !== 0) throw new Error(output);
  recordTransaction(`zk config ${args[args.indexOf("--nonce") + 1]}`, output);
  process.stdout.write(output);
}

function exportSwapVerificationKey() {
  const out = path.join(STATE, "tx_swap_verification_key.json");
  const result = spawnSync(
    "cargo",
    [
      "run",
      "--quiet",
      "--manifest-path",
      path.join(ROOT, "scripts/testnet-swap/vk-export/Cargo.toml"),
      "--",
      path.join(SWAP_ARTIFACTS, "tx_swap_proving_key.bin"),
      out,
    ],
    { encoding: "utf8", maxBuffer: 64 * 1024 * 1024 },
  );
  const output = `${result.stdout ?? ""}${result.stderr ?? ""}`;
  if (result.status !== 0) throw new Error(output);
  console.log(output.trim());
  return out;
}

ensureState();
stellar(["keys", "fund", "demo_user", "--network", "testnet"], {
  allowFailure: true,
});
const admin = stellar(["keys", "address", "demo_user"]).output.trim();
const kyt = Keypair.random();
const poolWasm = optimize("privacy_pools");
const registryWasm = optimize("private_address_registry");
const kytWasm = optimize("kyt_passage_registry");
const governanceWasm = optimize("privacy_pools_governance");

const governance = deploy(governanceWasm, "demo_user", ["--admin", admin]);
const pool = deploy(poolWasm, "demo_user", [
  "--tree_depth",
  "20",
  "--admin",
  admin,
  "--upgrader",
  governance,
]);
const registry = deploy(registryWasm, "demo_user", ["--admin", admin]);
const kytRegistry = deploy(kytWasm, "demo_user", [
  "--admin",
  admin,
  "--pool",
  pool,
  "--kyt_public_key",
  Buffer.from(kyt.rawPublicKey()).toString("hex"),
  "--max_ttl_ledgers",
  "500000",
]);

for (const [role, method, extra] of [
  ["set_registry", "set_registry", ["--registry", registry]],
  ["set_kyt_registry", "set_kyt_registry", ["--kyt_registry", kytRegistry]],
]) {
  const grant = stellar([
    "contract",
    "invoke",
    "--id",
    pool,
    "--source",
    "demo_user",
    "--network",
    "testnet",
    "--",
    "grant_role",
    "--account",
    admin,
    "--role",
    role,
    "--caller",
    admin,
  ]).output;
  recordTransaction(`grant ${role}`, grant);
  const set = stellar([
    "contract",
    "invoke",
    "--id",
    pool,
    "--source",
    "demo_user",
    "--network",
    "testnet",
    "--",
    method,
    ...extra,
    "--operator",
    admin,
  ]).output;
  recordTransaction(method, set);
}

writeJson("deploy.json", {
  pool,
  registry,
  kytRegistry,
  governance,
  admin,
  kytSignerSecret: kyt.secret(),
});

registerConfig([
  "--contract-id",
  pool,
  "--network",
  "testnet",
  "--admin",
  "demo_user",
  "--nonce",
  "7",
  "--vk-json",
  path.join(ARTIFACTS, "main_6x6_verification_key.json"),
  "--shape-json",
  path.join(ROOT, "shapes.json"),
  "--shape-id",
  "6x6",
]);
registerConfig([
  "--contract-id",
  pool,
  "--network",
  "testnet",
  "--admin",
  "demo_user",
  "--nonce",
  "8",
  "--vk-json",
  exportSwapVerificationKey(),
  "--n-ins",
  "6",
  "--n-outs",
  "6",
  "--public-n-inputs",
  "0",
  "--public-n-outputs",
  "0",
  "--n-audit-slots",
  "12",
  "--note-audit-len",
  "12",
  "--note-output-len",
  "7",
  "--audit-offset",
  "24",
  "--output-note-offset",
  "38",
]);

writeJson("deploy.json", {
  pool,
  registry,
  kytRegistry,
  governance,
  admin,
  kytSignerSecret: kyt.secret(),
});
console.log(`pool ${pool}`);
