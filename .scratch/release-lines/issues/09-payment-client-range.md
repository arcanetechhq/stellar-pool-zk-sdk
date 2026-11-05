# 09 — Payment client 0.x install window

**What to build:** The payment client installs the Stellar privacy package at `>=0.7.0 <1.0.0`. Patch and minor updates of the 0.x line arrive without another dependency edit. A 1.x Stellar package stays outside the install. The client still does not depend on the zk SDK directly.

**Blocked by:** None — can start immediately.

**Status:** done

- [x] The payment client depends on the Stellar privacy package at `>=0.7.0 <1.0.0`
- [x] A 1.x release of that package does not satisfy the dependency
- [x] The payment client does not declare a direct dependency on the zk SDK
