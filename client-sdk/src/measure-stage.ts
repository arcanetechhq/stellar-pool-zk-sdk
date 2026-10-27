const TX_BENCH_PREFIX = "arcane-tx:";

export async function measureStage<T>(
  name: string,
  run: () => Promise<T> | T,
): Promise<T> {
  const start = `${TX_BENCH_PREFIX}${name}:start`;
  performance.mark(start);
  try {
    return await run();
  } finally {
    const end = `${TX_BENCH_PREFIX}${name}:end`;
    performance.mark(end);
    performance.measure(`${TX_BENCH_PREFIX}${name}`, start, end);
  }
}
