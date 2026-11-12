# The binding nonce keeps its number

Binding circuits keep zk nonces 3, 7, and 10. The delegated template uses the same public layout. Six signals per input replace the single Outcome signal, so each vector grows by five signals per input: nonce 3 grows from 32 to 42, nonce 7 from 64 to 94, and nonce 10 from 47 to 97. A test stand deploys a new pool and registers the new 6×6 verifying key under nonce 7. An old pool keeps the previous key and cannot verify the new proof.

## Considered options

- Allocate nonce 8 for the new 6×6 vector. Rejected: tests deploy a new pool, so the existing nonce numbers stay.
