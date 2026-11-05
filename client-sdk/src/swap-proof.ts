import type { CoinData, StateFile, WithdrawMerkleWitness } from "./types";
import type { TransactionAuditParams } from "./transaction-audit";
import {
  randomFrDecimal,
  randomFrDecimal253,
} from "./withdrawal-transaction-input";

const TREE_DEPTH = 20;
const PADDING_SPEND_SCALAR = "13";

export interface SwapNoteOpening {
  value: string;
  nullifier: string;
  secret: string;
  commitment: string;
  asset_hi: string;
  asset_lo: string;
  application_id: string;
  accumulator: string;
  owner_x: string;
  owner_y: string;
  ephemeral_key_scalar: string;
}

export interface SwapAuthorizationResult {
  signature: { R8: [string, string]; S: string };
  signedFields: string[];
  minimumSenderBalance: string;
  bPaid: string;
  noteOpenings: SwapNoteOpening[];
}

export interface ProveSwapParams {
  authorization: SwapAuthorizationResult;
  sellerNotes: CoinData[];
  senderNotes: CoinData[];
  senderSpendScalar: string;
  state: StateFile;
  audit: TransactionAuditParams;
  /**
   * When set, proving stops once this ledger has reached the expiration ledger.
   * Omitted, the pool remains the authority on expiry.
   */
  currentLedger?: string;
  /** Acceptance harness writes the witness that was proved so snarkjs can check it. */
  witnessInputPath?: string;
}

export interface ExactSwapWitnessInput {
  stateRoot: string;
  expirationLedger: string;
  senderPrivKeyScalar: string;
  paddingPrivKeyScalars: string[];
  ownerPubs: [string, string][];
  paddingRandoms: string[];
  withdrawnValues: string[];
  withdrawnNullifiers: string[];
  withdrawnSecrets: string[];
  withdrawnAssets: [string, string][];
  inputApplicationIds: string[];
  withdrawnAccumulators: string[];
  withdrawnEscrowNonces: string[];
  inputRecipientStellar: [string, string][];
  stateSiblings: string[][];
  stateIndex: string[];
  depositedValues: string[];
  depositedAssets: [string, string][];
  depositedNullifiers: string[];
  outputApplicationIds: string[];
  depositedAccumulators: string[];
  depositedEphemeralKeyScalars: string[];
  depositedRecipientPublicKeys: [string, string][];
  depositedEscrowNonces: string[];
  outputRecipientStellar: [string, string][];
  quoteNum: string;
  quoteDen: string;
  swapAmount: string;
  bPaid: string;
  quoteRemainder: string;
  signatureR8: [string, string];
  signatureS: string;
  auditEphemeralScalar: string;
  noteAuditPublicKey: [string, string];
}

type OwnerPoint = [string, string];

function zeros(count: number): string[] {
  return Array.from({ length: count }, () => "0");
}

function zeroPoints(count: number): OwnerPoint[] {
  return Array.from({ length: count }, () => ["0", "0"]);
}

function signedField(
  authorization: SwapAuthorizationResult,
  index: number,
  name: string,
): string {
  const field = authorization.signedFields?.[index];
  if (typeof field !== "string" || !/^[0-9]+$/.test(field)) {
    throw new Error(`swap authorization is missing the ${name}`);
  }
  return field;
}

interface SwapQuote {
  quoteNum: string;
  quoteDen: string;
  swapAmount: string;
  expirationLedger: string;
  bPaid: string;
  quoteRemainder: string;
}

function readSwapQuote(authorization: SwapAuthorizationResult): SwapQuote {
  const quoteNum = signedField(authorization, 7, "quote numerator");
  const quoteDen = signedField(authorization, 8, "quote denominator");
  const swapAmount = signedField(authorization, 9, "swap amount");
  const expirationLedger = signedField(authorization, 10, "expiration ledger");
  const bPaid = authorization.bPaid;
  if (typeof bPaid !== "string" || !/^[0-9]+$/.test(bPaid)) {
    throw new Error("swap authorization is missing bPaid");
  }
  const product = BigInt(swapAmount) * BigInt(quoteNum);
  const paid = BigInt(bPaid) * BigInt(quoteDen);
  if (paid > product) {
    throw new Error("bPaid does not match the quote");
  }
  return {
    quoteNum,
    quoteDen,
    swapAmount,
    expirationLedger,
    bPaid,
    quoteRemainder: (product - paid).toString(10),
  };
}

function requireSignature(authorization: SwapAuthorizationResult) {
  const signature = authorization.signature;
  if (!signature?.R8 || signature.R8.length !== 2 || !signature.S) {
    throw new Error("swap authorization is missing its signature");
  }
  return signature;
}

interface PlacedNote {
  witness: WithdrawMerkleWitness;
  note: CoinData;
  owner: OwnerPoint;
  sender: boolean;
}

interface SwapInputLayout {
  stateRoot: string;
  senderPrivKeyScalar: string;
  paddingPrivKeyScalars: string[];
  ownerPubs: OwnerPoint[];
  paddingRandoms: string[];
  withdrawnValues: string[];
  withdrawnNullifiers: string[];
  withdrawnSecrets: string[];
  withdrawnAssets: OwnerPoint[];
  inputApplicationIds: string[];
  withdrawnAccumulators: string[];
  withdrawnEscrowNonces: string[];
  inputRecipientStellar: OwnerPoint[];
  stateSiblings: string[][];
  stateIndex: string[];
  senderAsset: OwnerPoint;
  sellerAsset: OwnerPoint;
  applicationId: string;
}

function layoutSwapInputs(parameters: {
  senderNotes: CoinData[];
  sellerNotes: CoinData[];
  senderWitnesses: WithdrawMerkleWitness[];
  sellerWitnesses: WithdrawMerkleWitness[];
  senderSpendScalar: string;
  senderOwner: OwnerPoint;
  sellerOwner: OwnerPoint;
  paddingOwner: OwnerPoint;
}): SwapInputLayout {
  const senderCount = parameters.senderNotes.length;
  const sellerCount = parameters.sellerNotes.length;
  if (
    senderCount < 1 ||
    senderCount > 5 ||
    sellerCount < 1 ||
    sellerCount > 5 ||
    senderCount + sellerCount > 6
  ) {
    throw new Error("a swap accepts one to five notes on each side");
  }
  if (
    parameters.senderWitnesses.length !== senderCount ||
    parameters.sellerWitnesses.length !== sellerCount
  ) {
    throw new Error("swap notes are missing merkle witnesses");
  }
  const placed: Array<PlacedNote | null> = Array.from({ length: 6 }, () => null);
  placed[0] = {
    witness: parameters.senderWitnesses[0],
    note: parameters.senderNotes[0],
    owner: parameters.senderOwner,
    sender: true,
  };
  placed[1] = {
    witness: parameters.sellerWitnesses[0],
    note: parameters.sellerNotes[0],
    owner: parameters.sellerOwner,
    sender: false,
  };
  let cursor = 2;
  for (let index = 1; index < sellerCount; index += 1) {
    placed[cursor] = {
      witness: parameters.sellerWitnesses[index],
      note: parameters.sellerNotes[index],
      owner: parameters.sellerOwner,
      sender: false,
    };
    cursor += 1;
  }
  for (let index = 1; index < senderCount; index += 1) {
    placed[cursor] = {
      witness: parameters.senderWitnesses[index],
      note: parameters.senderNotes[index],
      owner: parameters.senderOwner,
      sender: true,
    };
    cursor += 1;
  }
  const paddingPrivKeyScalars: string[] = [];
  const ownerPubs: OwnerPoint[] = [];
  const withdrawnValues: string[] = [];
  const withdrawnNullifiers: string[] = [];
  const withdrawnSecrets: string[] = [];
  const withdrawnAssets: OwnerPoint[] = [];
  const inputApplicationIds: string[] = [];
  const stateSiblings: string[][] = [];
  const stateIndex: string[] = [];
  for (let slot = 0; slot < 6; slot += 1) {
    const note = placed[slot];
    if (note) {
      ownerPubs.push(note.owner);
      withdrawnValues.push(note.witness.value);
      withdrawnNullifiers.push(note.witness.nullifier);
      withdrawnSecrets.push(note.witness.secret);
      withdrawnAssets.push(note.witness.withdrawnAsset);
      inputApplicationIds.push(noteApplicationId(note.note));
      stateSiblings.push(siblingsOf(note.witness));
      stateIndex.push(note.witness.stateIndex);
      if (slot >= 2) {
        paddingPrivKeyScalars.push(
          note.sender ? parameters.senderSpendScalar : PADDING_SPEND_SCALAR,
        );
      }
    } else {
      ownerPubs.push(parameters.paddingOwner);
      withdrawnValues.push("0");
      withdrawnNullifiers.push(randomFrDecimal());
      withdrawnSecrets.push(String(slot - 1));
      withdrawnAssets.push(["0", "0"]);
      inputApplicationIds.push("0");
      stateSiblings.push(zeros(TREE_DEPTH));
      stateIndex.push("0");
      if (slot >= 2) {
        paddingPrivKeyScalars.push(PADDING_SPEND_SCALAR);
      }
    }
  }
  return {
    stateRoot: parameters.senderWitnesses[0].stateRoot,
    senderPrivKeyScalar: parameters.senderSpendScalar,
    paddingPrivKeyScalars,
    ownerPubs,
    paddingRandoms: Array.from({ length: 6 }, () => randomFrDecimal()),
    withdrawnValues,
    withdrawnNullifiers,
    withdrawnSecrets,
    withdrawnAssets,
    inputApplicationIds,
    withdrawnAccumulators: zeros(6),
    withdrawnEscrowNonces: zeros(6),
    inputRecipientStellar: zeroPoints(6),
    stateSiblings,
    stateIndex,
    senderAsset: parameters.senderWitnesses[0].withdrawnAsset,
    sellerAsset: parameters.sellerWitnesses[0].withdrawnAsset,
    applicationId: noteApplicationId(parameters.sellerNotes[0]),
  };
}

function noteApplicationId(note: CoinData): string {
  return note.application_id && note.application_id.length > 0
    ? note.application_id
    : "0";
}

function siblingsOf(witness: WithdrawMerkleWitness): string[] {
  const siblings = witness.stateSiblings.map((entry) => entry.toString());
  if (siblings.length > TREE_DEPTH) {
    return siblings.slice(0, TREE_DEPTH);
  }
  return [...siblings, ...zeros(TREE_DEPTH - siblings.length)];
}

/**
 * Exact swap witness: one sender note, one seller note, the seller's proceeds
 * opening copied from the authorization, one new note for the sender, and padding.
 */
export function buildExactSwapWitnessInput(parameters: {
  authorization: SwapAuthorizationResult;
  sellerNotes: CoinData[];
  senderNotes: CoinData[];
  senderSpendScalar: string;
  audit: TransactionAuditParams;
  senderWitnesses: WithdrawMerkleWitness[];
  sellerWitnesses: WithdrawMerkleWitness[];
  senderOwner: OwnerPoint;
  paddingOwner: OwnerPoint;
}): ExactSwapWitnessInput {
  const { authorization, audit } = parameters;
  if (authorization.noteOpenings?.length !== 1) {
    throw new Error("an exact swap copies one proceeds opening");
  }
  if (!audit?.noteAuditPublicKey || !audit.auditEphemeralScalar) {
    throw new Error("proveSwap requires audit parameters");
  }
  const proceeds = authorization.noteOpenings[0];
  const quote = readSwapQuote(authorization);
  const sellerOwner: OwnerPoint = [proceeds.owner_x, proceeds.owner_y];
  const padding = parameters.paddingOwner;
  const signature = requireSignature(authorization);
  const inputs = layoutSwapInputs({
    senderNotes: parameters.senderNotes,
    sellerNotes: parameters.sellerNotes,
    senderWitnesses: parameters.senderWitnesses,
    sellerWitnesses: parameters.sellerWitnesses,
    senderSpendScalar: parameters.senderSpendScalar,
    senderOwner: parameters.senderOwner,
    sellerOwner,
    paddingOwner: padding,
  });
  const { senderAsset, sellerAsset, applicationId } = inputs;

  return {
    stateRoot: inputs.stateRoot,
    expirationLedger: quote.expirationLedger,
    senderPrivKeyScalar: inputs.senderPrivKeyScalar,
    paddingPrivKeyScalars: inputs.paddingPrivKeyScalars,
    ownerPubs: inputs.ownerPubs,
    paddingRandoms: inputs.paddingRandoms,
    withdrawnValues: inputs.withdrawnValues,
    withdrawnNullifiers: inputs.withdrawnNullifiers,
    withdrawnSecrets: inputs.withdrawnSecrets,
    withdrawnAssets: inputs.withdrawnAssets,
    inputApplicationIds: inputs.inputApplicationIds,
    withdrawnAccumulators: inputs.withdrawnAccumulators,
    withdrawnEscrowNonces: inputs.withdrawnEscrowNonces,
    inputRecipientStellar: inputs.inputRecipientStellar,
    stateSiblings: inputs.stateSiblings,
    stateIndex: inputs.stateIndex,
    depositedValues: [proceeds.value, quote.bPaid, ...zeros(4)],
    depositedAssets: [
      [proceeds.asset_hi, proceeds.asset_lo],
      sellerAsset,
      ...zeroPoints(4),
    ],
    depositedNullifiers: [
      proceeds.nullifier,
      randomFrDecimal(),
      randomFrDecimal(),
      randomFrDecimal(),
      randomFrDecimal(),
      randomFrDecimal(),
    ],
    outputApplicationIds: [proceeds.application_id, applicationId, ...zeros(4)],
    depositedAccumulators: [proceeds.accumulator, ...zeros(5)],
    depositedEphemeralKeyScalars: [
      proceeds.ephemeral_key_scalar,
      randomFrDecimal253(),
      randomFrDecimal253(),
      randomFrDecimal253(),
      randomFrDecimal253(),
      randomFrDecimal253(),
    ],
    depositedRecipientPublicKeys: [
      sellerOwner,
      parameters.senderOwner,
      padding,
      padding,
      padding,
      padding,
    ],
    depositedEscrowNonces: zeros(6),
    outputRecipientStellar: zeroPoints(6),
    quoteNum: quote.quoteNum,
    quoteDen: quote.quoteDen,
    swapAmount: quote.swapAmount,
    bPaid: quote.bPaid,
    quoteRemainder: quote.quoteRemainder,
    signatureR8: [signature.R8[0], signature.R8[1]],
    signatureS: signature.S,
    auditEphemeralScalar: audit.auditEphemeralScalar,
    noteAuditPublicKey: audit.noteAuditPublicKey,
  };
}

/**
 * Swap with leftover value. The seller's proceeds and change notes are the
 * openings from the swap authorization. The sender receives bPaid of the
 * seller's asset and one change note of their own asset.
 */
export function buildChangeSwapWitnessInput(parameters: {
  authorization: SwapAuthorizationResult;
  sellerNotes: CoinData[];
  senderNotes: CoinData[];
  senderSpendScalar: string;
  audit: TransactionAuditParams;
  senderWitnesses: WithdrawMerkleWitness[];
  sellerWitnesses: WithdrawMerkleWitness[];
  senderOwner: OwnerPoint;
  paddingOwner: OwnerPoint;
}): ExactSwapWitnessInput {
  const { authorization, audit } = parameters;
  if (authorization.noteOpenings?.length !== 2) {
    throw new Error("a swap with change copies the proceeds opening and the change opening");
  }
  if (!audit?.noteAuditPublicKey || !audit.auditEphemeralScalar) {
    throw new Error("proveSwap requires audit parameters");
  }
  const proceeds = authorization.noteOpenings[0];
  const sellerChange = authorization.noteOpenings[1];
  const quote = readSwapQuote(authorization);
  const senderChange = parameters.senderWitnesses.reduce(
    (sum, witness) => sum + BigInt(witness.value),
    0n,
  ) - BigInt(quote.swapAmount);
  const sellerOwner: OwnerPoint = [proceeds.owner_x, proceeds.owner_y];
  const padding = parameters.paddingOwner;
  const signature = requireSignature(authorization);
  const inputs = layoutSwapInputs({
    senderNotes: parameters.senderNotes,
    sellerNotes: parameters.sellerNotes,
    senderWitnesses: parameters.senderWitnesses,
    sellerWitnesses: parameters.sellerWitnesses,
    senderSpendScalar: parameters.senderSpendScalar,
    senderOwner: parameters.senderOwner,
    sellerOwner,
    paddingOwner: padding,
  });
  const { senderAsset, sellerAsset, applicationId } = inputs;
  const senderNote = parameters.senderNotes[0];

  return {
    stateRoot: inputs.stateRoot,
    expirationLedger: quote.expirationLedger,
    senderPrivKeyScalar: inputs.senderPrivKeyScalar,
    paddingPrivKeyScalars: inputs.paddingPrivKeyScalars,
    ownerPubs: inputs.ownerPubs,
    paddingRandoms: inputs.paddingRandoms,
    withdrawnValues: inputs.withdrawnValues,
    withdrawnNullifiers: inputs.withdrawnNullifiers,
    withdrawnSecrets: inputs.withdrawnSecrets,
    withdrawnAssets: inputs.withdrawnAssets,
    inputApplicationIds: inputs.inputApplicationIds,
    withdrawnAccumulators: inputs.withdrawnAccumulators,
    withdrawnEscrowNonces: inputs.withdrawnEscrowNonces,
    inputRecipientStellar: inputs.inputRecipientStellar,
    stateSiblings: inputs.stateSiblings,
    stateIndex: inputs.stateIndex,
    depositedValues: [
      proceeds.value,
      quote.bPaid,
      sellerChange.value,
      senderChange.toString(10),
      ...zeros(2),
    ],
    depositedAssets: [
      [proceeds.asset_hi, proceeds.asset_lo],
      sellerAsset,
      [sellerChange.asset_hi, sellerChange.asset_lo],
      senderAsset,
      ...zeroPoints(2),
    ],
    depositedNullifiers: [
      proceeds.nullifier,
      randomFrDecimal(),
      sellerChange.nullifier,
      randomFrDecimal(),
      randomFrDecimal(),
      randomFrDecimal(),
    ],
    outputApplicationIds: [
      proceeds.application_id,
      applicationId,
      sellerChange.application_id,
      noteApplicationId(senderNote),
      ...zeros(2),
    ],
    depositedAccumulators: [
      proceeds.accumulator,
      "0",
      sellerChange.accumulator,
      ...zeros(3),
    ],
    depositedEphemeralKeyScalars: [
      proceeds.ephemeral_key_scalar,
      randomFrDecimal253(),
      sellerChange.ephemeral_key_scalar,
      randomFrDecimal253(),
      randomFrDecimal253(),
      randomFrDecimal253(),
    ],
    depositedRecipientPublicKeys: [
      sellerOwner,
      parameters.senderOwner,
      [sellerChange.owner_x, sellerChange.owner_y],
      parameters.senderOwner,
      padding,
      padding,
    ],
    depositedEscrowNonces: zeros(6),
    outputRecipientStellar: zeroPoints(6),
    quoteNum: quote.quoteNum,
    quoteDen: quote.quoteDen,
    swapAmount: quote.swapAmount,
    bPaid: quote.bPaid,
    quoteRemainder: quote.quoteRemainder,
    signatureR8: [signature.R8[0], signature.R8[1]],
    signatureS: signature.S,
    auditEphemeralScalar: audit.auditEphemeralScalar,
    noteAuditPublicKey: audit.noteAuditPublicKey,
  };
}

export const EXACT_SWAP_PADDING_SCALAR = PADDING_SPEND_SCALAR;
