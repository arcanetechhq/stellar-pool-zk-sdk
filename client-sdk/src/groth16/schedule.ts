const MAX_WORKERS = 8;
const MIN_SCALARS_PER_WORKER = 16_384;

export type ProverEnvironment = "browser" | "node";

/** How many MSM workers to start for this device and circuit. */
export function workerCount(input: {
  cores: number;
  environment: ProverEnvironment;
  scalarCount: number;
}): number {
  const reserve = input.environment === "browser" ? 1 : 0;
  const usableCores = Math.max(1, Math.floor(input.cores) - reserve);
  const byWork = Math.max(
    1,
    Math.floor(input.scalarCount / MIN_SCALARS_PER_WORKER),
  );
  return Math.min(MAX_WORKERS, usableCores, byWork);
}
