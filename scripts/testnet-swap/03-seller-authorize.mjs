import fs from "node:fs";
import {
  createSwapAuthorization,
  initSync,
} from "../../client-sdk/pkg/client_sdk_wasm.js";
import { latestLedger, readJson, writeJson } from "./lib.mjs";

initSync({
  module: fs.readFileSync(
    new URL("../../client-sdk/pkg/client_sdk_wasm_bg.wasm", import.meta.url),
  ),
});

const seller = readJson("seller-secret.json");
const note = readJson("seller-note.json");
const sender = readJson("sender-public.json");
const expirationLedger = (await latestLedger()) + 2000;
const authorization = createSwapAuthorization(
  seller.spendScalar,
  JSON.stringify([note]),
  sender.ownerX,
  sender.ownerY,
  sender.assetHi,
  sender.assetLo,
  "1",
  "1",
  note.value,
  String(expirationLedger),
);
writeJson("authorization.json", {
  authorization,
  sellerNotes: [note],
});
console.log(`authorization expires at ledger ${expirationLedger}`);
