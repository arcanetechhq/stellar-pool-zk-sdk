import assert from "node:assert/strict";
import { workerCount } from "../dist/groth16-schedule.mjs";

const scalars = 126_000;

assert.equal(
  workerCount({ cores: 12, environment: "node", scalarCount: scalars }),
  7,
);
assert.equal(
  workerCount({ cores: 8, environment: "browser", scalarCount: scalars }),
  7,
);
assert.equal(
  workerCount({ cores: 4, environment: "browser", scalarCount: scalars }),
  3,
);
assert.equal(
  workerCount({ cores: 12, environment: "node", scalarCount: 100 }),
  1,
);

console.log("groth16-schedule: ok");
