import type {
  Groth16Pool,
  MsmJob,
  MsmRequest,
  WorkerOutbound,
} from "./protocol";
import type { ProverEnvironment } from "./schedule";

const START_TIMEOUT_MS = 60_000;

export interface PoolPort {
  post(message: object, transfer?: ArrayBuffer[]): void;
  close(): void;
  listen(handler: (message: WorkerOutbound) => void): void;
  fail(handler: (error: Error) => void): void;
  ref(): void;
  unref(): void;
}

interface LiveWorker {
  port: PoolPort;
  busy: boolean;
  cached: Set<string>;
  onMessage: (message: WorkerOutbound) => void;
  onError: (error: Error) => void;
}

export function createMsmPool(input: {
  environment: ProverEnvironment;
  cores: number;
  spawn(): PoolPort;
}): Groth16Pool {
  const workers: LiveWorker[] = [];
  let starting: Promise<void> | null = null;
  let disposed = false;

  function ensure(count: number): Promise<void> {
    if (workers.length >= count) {
      return Promise.resolve();
    }
    if (starting) {
      return starting.then(() => ensure(count));
    }
    starting = (async () => {
      while (workers.length < count) {
        if (disposed) {
          throw new Error("Groth16 pool is closed");
        }
        workers.push(await boot(input.spawn()));
      }
    })().finally(() => {
      starting = null;
    });
    return starting;
  }

  return {
    environment: input.environment,
    cores: input.cores,
    run(workerCount: number, jobs: MsmJob[]): Promise<Map<number, Uint8Array>> {
      return ensure(workerCount).then(() =>
        dispatch(workers.slice(0, workerCount), jobs),
      );
    },
    terminate(): void {
      disposed = true;
      for (const worker of workers) {
        worker.port.close();
      }
      workers.length = 0;
    },
  };
}

function boot(port: PoolPort): Promise<LiveWorker> {
  return new Promise((resolve, reject) => {
    const worker: LiveWorker = {
      port,
      busy: false,
      cached: new Set(),
      onMessage: () => {},
      onError: () => {},
    };
    port.listen((message) => {
      worker.onMessage(message);
    });
    port.fail((error) => {
      worker.onError(error);
    });
    const timeout = setTimeout(() => {
      reject(new Error("Groth16 worker did not start"));
    }, START_TIMEOUT_MS);
    worker.onError = (error) => {
      clearTimeout(timeout);
      reject(error);
    };
    worker.onMessage = (message) => {
      if (message.type !== "ready") {
        return;
      }
      clearTimeout(timeout);
      worker.onMessage = () => {};
      worker.onError = () => {};
      port.unref();
      resolve(worker);
    };
    port.post({ type: "init" });
  });
}

function dispatch(
  workers: LiveWorker[],
  jobs: MsmJob[],
): Promise<Map<number, Uint8Array>> {
  return new Promise((resolve, reject) => {
    const pending = [...jobs];
    const results = new Map<number, Uint8Array>();
    let failed = false;

    const settle = () => {
      for (const worker of workers) {
        worker.port.unref();
      }
    };

    const fail = (error: Error) => {
      if (failed) {
        return;
      }
      failed = true;
      settle();
      reject(error);
    };

    const pump = () => {
      if (failed) {
        return;
      }
      if (results.size === jobs.length) {
        settle();
        resolve(results);
        return;
      }
      assignCached(workers, pending, send);
      assignFresh(workers, pending, send);
    };

    const send = (worker: LiveWorker, job: MsmJob) => {
      worker.busy = true;
      const scalars = copyBytes(job.scalars);
      const transfer = [scalars];
      const message: MsmRequest = {
        type: "msm",
        id: job.id,
        curve: job.curve,
        cacheKey: job.cacheKey,
        scalars,
      };
      if (!worker.cached.has(job.cacheKey)) {
        const bases = copyBytes(job.bases);
        message.bases = bases;
        transfer.push(bases);
        worker.cached.add(job.cacheKey);
      }
      worker.port.post(message, transfer);
    };

    for (const worker of workers) {
      worker.onMessage = (message) => {
        if (message.type === "ready") {
          return;
        }
        worker.busy = false;
        if (message.type === "error") {
          fail(new Error(message.message));
          return;
        }
        results.set(message.id, new Uint8Array(message.point));
        pump();
      };
      worker.onError = fail;
    }
    for (const worker of workers) {
      worker.port.ref();
    }
    if (jobs.length === 0) {
      settle();
      resolve(results);
      return;
    }
    pump();
  });
}

function assignCached(
  workers: LiveWorker[],
  pending: MsmJob[],
  send: (worker: LiveWorker, job: MsmJob) => void,
): void {
  for (const worker of workers) {
    if (worker.busy) {
      continue;
    }
    const index = pending.findIndex((job) => worker.cached.has(job.cacheKey));
    if (index < 0) {
      continue;
    }
    const [job] = pending.splice(index, 1);
    if (job) {
      send(worker, job);
    }
  }
}

function assignFresh(
  workers: LiveWorker[],
  pending: MsmJob[],
  send: (worker: LiveWorker, job: MsmJob) => void,
): void {
  for (const worker of workers) {
    if (worker.busy) {
      continue;
    }
    const index = pending.findIndex(
      (job) => !workers.some((candidate) => candidate.cached.has(job.cacheKey)),
    );
    if (index < 0) {
      return;
    }
    const [job] = pending.splice(index, 1);
    if (job) {
      send(worker, job);
    }
  }
}

function copyBytes(bytes: Uint8Array): ArrayBuffer {
  return bytes.slice().buffer;
}
