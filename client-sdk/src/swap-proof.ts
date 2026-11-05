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
  senderWitness: WithdrawMerkleWitness;
  sellerWitness: WithdrawMerkleWitness;
  senderOwner: OwnerPoint;
  paddingOwner: OwnerPoint;
}): ExactSwapWitnessInput {
  const { authorization, audit } = parameters;
  if (parameters.senderNotes.length !== 1 || parameters.sellerNotes.length !== 1) {
    throw new Error("an exact swap spends one note on each side");
  }
  if (authorization.noteOpenings?.length !== 1) {
    throw new Error("an exact swap copies one proceeds opening");
  }
  if (!audit?.noteAuditPublicKey || !audit.auditEphemeralScalar) {
    throw new Error("proveSwap requires audit parameters");
  }
  const proceeds = authorization.noteOpenings[0];
  const senderNote = parameters.senderNotes[0];
  const sellerNote = parameters.sellerNotes[0];
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
  const senderAsset: OwnerPoint = [
    parameters.senderWitness.withdrawnAsset[0],
    parameters.senderWitness.withdrawnAsset[1],
  ];
  const sellerAsset: OwnerPoint = [
    parameters.sellerWitness.withdrawnAsset[0],
    parameters.sellerWitness.withdrawnAsset[1],
  ];
  const sellerOwner: OwnerPoint = [proceeds.owner_x, proceeds.owner_y];
  const padding = parameters.paddingOwner;
  const applicationId = noteApplicationId(sellerNote);
  const signature = authorization.signature;
  if (!signature?.R8 || signature.R8.length !== 2 || !signature.S) {
    throw new Error("swap authorization is missing its signature");
  }

  return {
    stateRoot: parameters.senderWitness.stateRoot,
    expirationLedger,
    senderPrivKeyScalar: parameters.senderSpendScalar,
    paddingPrivKeyScalars: [
      PADDING_SPEND_SCALAR,
      PADDING_SPEND_SCALAR,
      PADDING_SPEND_SCALAR,
      PADDING_SPEND_SCALAR,
    ],
    ownerPubs: [
      parameters.senderOwner,
      sellerOwner,
      padding,
      padding,
      padding,
      padding,
    ],
    paddingRandoms: [
      randomFrDecimal(),
      randomFrDecimal(),
      randomFrDecimal(),
      randomFrDecimal(),
      randomFrDecimal(),
      randomFrDecimal(),
    ],
    withdrawnValues: [
      parameters.senderWitness.value,
      parameters.sellerWitness.value,
      ...zeros(4),
    ],
    withdrawnNullifiers: [
      parameters.senderWitness.nullifier,
      parameters.sellerWitness.nullifier,
      randomFrDecimal(),
      randomFrDecimal(),
      randomFrDecimal(),
      randomFrDecimal(),
    ],
    withdrawnSecrets: [
      parameters.senderWitness.secret,
      parameters.sellerWitness.secret,
      "1",
      "2",
      "3",
      "4",
    ],
    withdrawnAssets: [senderAsset, sellerAsset, ...zeroPoints(4)],
    inputApplicationIds: [
      noteApplicationId(senderNote),
      applicationId,
      ...zeros(4),
    ],
    withdrawnAccumulators: zeros(6),
    withdrawnEscrowNonces: zeros(6),
    inputRecipientStellar: zeroPoints(6),
    stateSiblings: [
      siblingsOf(parameters.senderWitness),
      siblingsOf(parameters.sellerWitness),
      zeros(TREE_DEPTH),
      zeros(TREE_DEPTH),
      zeros(TREE_DEPTH),
      zeros(TREE_DEPTH),
    ],
    stateIndex: [
      parameters.senderWitness.stateIndex,
      parameters.sellerWitness.stateIndex,
      ...zeros(4),
    ],
    depositedValues: [proceeds.value, bPaid, ...zeros(4)],
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
    quoteNum,
    quoteDen,
    swapAmount,
    bPaid,
    quoteRemainder: (product - paid).toString(10),
    signatureR8: [signature.R8[0], signature.R8[1]],
    signatureS: signature.S,
    auditEphemeralScalar: audit.auditEphemeralScalar,
    noteAuditPublicKey: audit.noteAuditPublicKey,
  };
}

export const EXACT_SWAP_PADDING_SCALAR = PADDING_SPEND_SCALAR;
