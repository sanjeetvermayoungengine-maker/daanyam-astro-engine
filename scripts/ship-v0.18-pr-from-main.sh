#!/usr/bin/env bash
#
# Lifts the dasha_v2 timeline + yogas scaffold off `phase0-observability-baseline`
# onto a fresh branch from `main`. Runs everything UP TO `git commit`. Does not
# commit. Does not push. Does run `cargo check` + `cargo test` and refuses to
# proceed if either fails.
#
# Usage:  bash scripts/ship-v0.18-pr-from-main.sh
#         bash scripts/ship-v0.18-pr-from-main.sh --dry-run
#
# What it does:
#   1. Clears any stale .git/index.lock.
#   2. Stashes the V0.18 work (4 modified + yogas/ + docs/engine/) with
#      --include-untracked so the planning docs etc. ride along to be re-popped.
#   3. Switches to `main`, pulls latest, creates v0.18-dasha-timeline-and-yogas.
#   4. Pops the stash onto the new branch.
#   5. Selectively `git add`s ONLY the six dasha_v2 + yogas paths. Leaves
#      business-site/, AstroEngine_LaunchPlan.docx, etc. untracked.
#   6. Runs `cargo check --workspace --all-features` and `cargo test --workspace
#      --all-features`. Refuses to continue on red.
#   7. Prints the staged diff so you can review before committing.

set -euo pipefail

DRY_RUN="${1:-}"
ADD_CMD=( git add )
[[ "${DRY_RUN}" == "--dry-run" ]] && ADD_CMD=( echo "(dry-run) would: git add" )

# Mutates repo, index, or working tree. Skipped in --dry-run (print only).
git_mutate() {
  if [[ "${DRY_RUN}" == "--dry-run" ]]; then
    printf "(dry-run) would:"
    printf " %q" "$@"
    printf "\n"
    return 0
  fi
  "$@"
}

log() { printf "\n\033[1;36m▸ %s\033[0m\n" "$*"; }
warn() { printf "\n\033[1;33m! %s\033[0m\n" "$*"; }
die()  { printf "\n\033[1;31m✗ %s\033[0m\n" "$*" >&2; exit 1; }

cd "$(git rev-parse --show-toplevel)"

NEW_BRANCH="v0.18-dasha-timeline-and-yogas"
SOURCE_BRANCH="phase0-observability-baseline"
STASH_MSG="v0.18: dasha_v2 timeline + yogas scaffold"

# ─── Step 1: lockfile + source-branch checks ─────────────────────────────────

log "Step 1/7 — Lockfile + source-branch checks"

if [[ -f ".git/index.lock" ]]; then
  warn "Found stale .git/index.lock — removing."
  git_mutate rm -f .git/index.lock
fi

CURRENT_BRANCH=$(git rev-parse --abbrev-ref HEAD)
if [[ "${CURRENT_BRANCH}" != "${SOURCE_BRANCH}" ]]; then
  warn "Expected to start on '${SOURCE_BRANCH}', currently on '${CURRENT_BRANCH}'."
  warn "If your V0.18 work lives elsewhere, abort and rerun manually."
  read -p "Continue anyway? [y/N] " yn
  [[ "${yn}" =~ ^[Yy]$ ]] || die "Aborted by user."
fi

# Sanity-check that the expected modifications are present
EXPECTED=(
  crates/astro-api/src/lib.rs
  crates/astro-vedic/src/dasha.rs
  crates/astro-vedic/src/lib.rs
  dist/openapi.json
)
for f in "${EXPECTED[@]}"; do
  if ! git diff --name-only | grep -qx "$f"; then
    warn "Expected modification not detected in working tree: $f"
    warn "If you already committed it, this script may stage nothing meaningful."
  fi
done

YOGA_DIR="crates/astro-vedic/src/yogas"
if [[ ! -d "${YOGA_DIR}" ]]; then
  die "Yoga scaffold directory ${YOGA_DIR}/ not found — aborting."
fi

# ─── Step 2: stash with untracked ────────────────────────────────────────────

log "Step 2/7 — Stash V0.18 work (incl. untracked)"

git_mutate git stash push --include-untracked --message "${STASH_MSG}"

# ─── Step 3: switch to main + pull + new branch ──────────────────────────────

log "Step 3/7 — Switch to main, pull, create ${NEW_BRANCH}"

git_mutate git checkout main
git_mutate git pull origin main --ff-only

if git rev-parse --verify --quiet "${NEW_BRANCH}" >/dev/null; then
  warn "Branch ${NEW_BRANCH} already exists. Checking out."
  git_mutate git checkout "${NEW_BRANCH}"
else
  git_mutate git checkout -b "${NEW_BRANCH}"
fi

# ─── Step 4: pop the stash ───────────────────────────────────────────────────

log "Step 4/7 — Pop the V0.18 stash"

if [[ "${DRY_RUN}" == "--dry-run" ]]; then
  echo "(dry-run) would: git stash pop"
elif ! git stash pop; then
  die "Stash pop failed — resolve conflicts manually, then run cargo check + test."
fi

# ─── Step 5: selective staging ───────────────────────────────────────────────

log "Step 5/7 — Stage dasha_v2 + yogas paths only"

V018_PATHS=(
  crates/astro-api/src/lib.rs
  crates/astro-vedic/src/dasha.rs
  crates/astro-vedic/src/lib.rs
  crates/astro-vedic/src/yogas
  dist/openapi.json
  docs/engine/yogas-roadmap.md
)
for p in "${V018_PATHS[@]}"; do
  if [[ -e "$p" ]]; then
    "${ADD_CMD[@]}" "$p"
  else
    warn "Expected path not present: $p"
  fi
done

# ─── Step 6: cargo check + test ──────────────────────────────────────────────

log "Step 6/7 — cargo check + cargo test (sandbox-free verification)"

if [[ "${DRY_RUN}" != "--dry-run" ]]; then
  cargo check --workspace --all-features
  cargo test  --workspace --all-features --no-fail-fast
fi

# ─── Step 7: summary ─────────────────────────────────────────────────────────

log "Step 7/7 — Staged-vs-unstaged summary"

if [[ "${DRY_RUN}" == "--dry-run" ]]; then
  echo
  echo "(dry-run) Skipped staged/unstaged summary — no checkout, stash pop, or git add ran."
  echo
  log "Dry-run complete (no repo mutations)."
  echo "Next: rerun without --dry-run, then follow the post-run steps."
else
  echo
  echo "── STAGED for the v0.18 PR ──"
  git diff --cached --stat | tail -40

  echo
  echo "── UNTRACKED / unstaged (left out by design: business-site/, planning docs, etc.) ──"
  git status --short | grep -vE '^[AMRD] ' | head -40

  echo
  log "Done staging on branch ${NEW_BRANCH}."
  echo
  echo "Next:"
  echo "  1. Review:    git diff --cached"
  echo "  2. Commit:    use the message from your engine ship plan §2.7 (feat(engine v0.18): …)"
  echo "  3. Push:      git push -u origin ${NEW_BRANCH}"
  echo "  4. PR:        open a PR targeting main on GitHub"
  echo
  echo "Optional cleanup after the PR lands and the engine redeploys:"
  echo "  cargo run --bin astro-api  &  # local engine"
  echo "  curl -sS http://localhost:8080/openapi.json | jq . > dist/openapi.json"
  echo "  git diff dist/openapi.json    # confirm the regenerated file matches your intent"
fi
