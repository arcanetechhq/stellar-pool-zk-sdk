# 08 — Privacy SDK CI on both release lines

**What to build:** Pull requests rehearse the privacy SDK release with the same script publishers run locally. A push to the 0.x or 1.x branch publishes for real through that script. Core, state, and relay still publish only from the 0.x branch. Release automation follows those two branches rather than a single main branch.

**Blocked by:** 05 — Privacy SDK 0.x release plan and zk SDK range; 06 — Privacy SDK 1.x release plan; 07 — Privacy SDK protected merge verdict

**Status:** done

- [x] A pull request runs the release script as a dry-run and does not publish
- [x] A push to the 0.x branch publishes the stable 0.x versions the script selects, including core, state, and relay when they have a new version
- [x] A push to the 1.x branch publishes the Stellar package version the script selects and does not publish core, state, or relay
- [x] A manual promotion on the 1.x branch publishes stable Stellar `1.0.0` and moves `latest`
- [x] Release automation is triggered from the 0.x and 1.x branches
