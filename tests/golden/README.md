# Golden Tests

This directory holds stable regression fixtures that back deployment confidence checks.

Today the golden coverage is split across two layers:

- local fixture files such as [lahiri_moon_nakshatra.json](/Users/sanjeet/Desktop/daanyam-astroengine/tests/golden/lahiri_moon_nakshatra.json), used by crate tests for deterministic sidereal math
- deployed endpoint verification via [`crates/astro-api/tests/production_contract.rs`](/Users/sanjeet/Desktop/daanyam-astroengine/crates/astro-api/tests/production_contract.rs), which hits a live base URL and asserts the production HTTP contract stays on the DE440-backed path

Run the deployed verification suite with:

```bash
ASTRO_API_BASE_URL="https://your-cloud-run-url" cargo test -p astro-api --test production_contract
```

This is the Phase 0 go-live gate after Cloud Run deployment:

- `GET /health` returns `200`
- `/positions/sidereal` returns DE440-backed computation metadata
- `/chart/sidereal` returns the expected compact mobile contract shape
