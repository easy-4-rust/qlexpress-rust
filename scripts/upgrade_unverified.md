# upgrade_unverified.py -- UNVERIFIED Upgrade Automation

Automate the manual UNVERIFIED upgrade workflow when upstream QLExpress4
publishes a new release.  Turns seven hand-run steps into a single
reproducible, auditable command.

## Quick Start

When upstream QLExpress4 publishes a new version:

```bash
# 1. Check what needs upgrading (read-only, no changes)
python3 scripts/upgrade_unverified.py check \
    --java-repo /path/to/QLExpress \
    --java-baseline-sha <new-40-hex-sha>

# 2. Run the full upgrade
python3 scripts/upgrade_unverified.py apply \
    --java-repo /path/to/QLExpress \
    --java-baseline-sha <new-40-hex-sha>

# 3. Verify the result
python3 scripts/upgrade_unverified.py check \
    --java-repo /path/to/QLExpress \
    --java-baseline-sha <new-40-hex-sha>

# 4. Clean up the worktree
python3 scripts/upgrade_unverified.py clean \
    --java-repo /path/to/QLExpress \
    --java-baseline-sha <new-40-hex-sha>
```

## Subcommands

### `check` -- Read-only audit

Runs steps 1-5 without modifying any files.  Reports:

- Current UNVERIFIED/MISSING method counts
- Owner type distribution of unhandled methods
- Dispositions file health (schema, baselines, fingerprint)

```bash
python3 scripts/upgrade_unverified.py check \
    --java-repo /Users/me/QLExpress \
    --java-baseline-sha 9065b9ac5d985dcd02e627239aa9cdb78fb2f7f3
```

### `apply` -- Full upgrade

Runs the complete 7-step workflow:

1. **Worktree**: Create (or reuse) a git worktree at the baseline SHA
2. **CodeGraph**: Index both Java worktree and Rust tree
3. **Test inventory**: Generate or reuse test inventory
4. **Fingerprint**: Compute and validate the Rust source fingerprint
5. **Manifest**: Run `generate_migration_manifest.py` with validated paths
6. **Commit**: Move temp artifacts to committed locations
7. **Clean temp**: Remove leftover temporary files

```bash
python3 scripts/upgrade_unverified.py apply \
    --java-repo /Users/me/QLExpress \
    --java-baseline-sha 9065b9ac5d985dcd02e627239aa9cdb78fb2f7f3
```

**Idempotent**: Running `apply` twice with unchanged sources produces
identical output.  An existing worktree at the correct SHA is reused.

**Recoverable**: If manifest generation fails, the committed manifest
pair is not touched.  Temp files are cleaned up automatically.

### `clean` -- Remove worktree and temp files

```bash
python3 scripts/upgrade_unverified.py clean \
    --java-repo /Users/me/QLExpress \
    --java-baseline-sha 9065b9ac5d985dcd02e627239aa9cdb78fb2f7f3
```

## Parameters

| Parameter | Default | Description |
|-----------|---------|-------------|
| `--java-repo` | (required) | Path to the QLExpress Java repository |
| `--java-baseline-sha` | (required) | 40-hex commit SHA for the baseline |
| `--rust-root` | `.` | Path to the qlexpress-rust repository |
| `--worktree-root` | `/tmp` | Directory for temporary worktrees |
| `--rust-source-root` | `crates/qlexpress/src` | Rust source root relative to rust-root |
| `--java-package-root-suffix` | `src/main/java/com/alibaba/qlexpress4` | Java package root relative to worktree |

### `apply`-only parameters

| Parameter | Default | Description |
|-----------|---------|-------------|
| `--test-inventory-cmd` | None | Shell command to generate test inventory |
| `--skip-codegraph` | False | Skip CodeGraph indexing step |
| `--skip-test-inventory` | False | Skip test inventory generation |

## Output Format

### `check` output example

```
  UNVERIFIED UPGRADE CHECK
  Java repo:    /Users/me/QLExpress
  Rust root:    /Users/me/qlexpress-rust
  Baseline SHA: 9065b9ac5d985dcd02e627239aa9cdb78fb2f7f3
  Worktree:     /tmp/qlx-baseline-9065b9ac

  Current manifest summary:

  object_states (237 total):
    IMPLEMENTED            237  (100.0%)  ########################################

  method_states (1814 total):
    IMPLEMENTED           1811  ( 99.8%)  ########################################
    PLATFORM_NA              3  (  0.2%)  #

  Summary: 1814/1814 methods handled, 0 UNVERIFIED, 0 MISSING

  Dispositions file: .../migration-dispositions.json
    schema_version:     1
    java_baseline:      9065b9ac5d98...
    rust_source_fp:     sha256:b26c3b13afa3...
    objects: 237 entries
    nested_java_types: 136 entries
    methods: 1811 entries
```

### `apply` step summary example

```
  STEP SUMMARY
  ============================================================
  Step                           Status     Time
  ------------------------------ -------- --------
  validate_fingerprint           OK         0.3s
  setup_worktree                 OK         0.5s
  index_java_codegraph           OK        12.4s
  index_rust_codegraph           OK         8.1s
  generate_test_inventory        OK         3.2s
  generate_manifest              OK        45.6s
  commit_artifacts               OK         0.0s
  cleanup_temp                   OK         0.0s
  ------------------------------ -------- --------
  TOTAL                                   70.1s
```

## Prerequisites

- Python 3.10+ (stdlib only, no third-party packages)
- `git` on PATH
- `codegraph` on PATH (for indexing step)
- QLExpress Java repository cloned locally
- `scripts/generate_migration_manifest.py` in the rust root

## Workflow: After Upstream QLExpress4 Publishes

1. **Get the new commit SHA** from the QLExpress4 repository.

2. **Update `Cargo.toml`** workspace metadata:
   ```toml
   [workspace.metadata.qlexpress]
   java-baseline-commit = "<new-sha>"
   ```

3. **Run `check`** to see the current state:
   ```bash
   python3 scripts/upgrade_unverified.py check \
       --java-repo /path/to/QLExpress \
       --java-baseline-sha <new-sha>
   ```

4. **If dispositions are stale** (fingerprint mismatch), update
   `verification/migration-dispositions.json` with new evidence before
   proceeding.

5. **Run `apply`**:
   ```bash
   python3 scripts/upgrade_unverified.py apply \
       --java-repo /path/to/QLExpress \
       --java-baseline-sha <new-sha>
   ```

6. **Verify** with another `check` and inspect the generated manifest.

7. **Clean up**:
   ```bash
   python3 scripts/upgrade_unverified.py clean \
       --java-repo /path/to/QLExpress \
       --java-baseline-sha <new-sha>
   ```

## Troubleshooting

### Worktree already exists at wrong SHA

The script detects this automatically and removes the stale worktree
before creating a new one.  If manual cleanup is needed:

```bash
git -C /path/to/QLExpress worktree remove --force /tmp/qlx-baseline-<sha>
rm -rf /tmp/qlx-baseline-<sha>
```

### CodeGraph indexing fails

Ensure `codegraph` is installed and on PATH:

```bash
codegraph --version
```

If the Java worktree's CodeGraph database is corrupted:

```bash
rm -rf /tmp/qlx-baseline-<sha>/.codegraph
python3 scripts/upgrade_unverified.py apply --skip-test-inventory ...
```

### Fingerprint mismatch

This means the Rust source tree has changed since the dispositions
file was last validated.  Options:

1. **Re-validate dispositions**: Run the disposition validator to
   re-approve the changed sources against the current fingerprint.

2. **Regenerate dispositions**: If the changes are extensive, rebuild
   `migration-dispositions.json` from scratch.

3. **Skip fingerprint check**: Not recommended.  The fingerprint
   ensures dispositions are applied to the correct source tree.

### Manifest generation fails

Check the error output for the specific failure in
`generate_migration_manifest.py`.  Common causes:

- Missing CodeGraph databases (run without `--skip-codegraph`)
- Stale dispositions (fingerprint mismatch)
- Java package root mismatch (adjust `--java-package-root-suffix`)

Temp files are cleaned up automatically on failure.

### Test inventory missing

If `verification/migration-test-inventory-current.json` does not exist
and no `--test-inventory-cmd` is provided, the `apply` step will abort.

Provide a command to generate it:

```bash
python3 scripts/upgrade_unverified.py apply \
    --test-inventory-cmd "python3 scripts/generate_test_inventory.py" \
    ...
```

Or use `--skip-test-inventory` to proceed without it (the manifest
generator accepts `--test-inventory` as optional).

## Design Notes

### Why a separate script?

The upgrade workflow involves seven steps with specific ordering
constraints and error-handling requirements.  Encoding this in a
shell script would lose structured error reporting and testability.
A Python script provides:

- Structured step results with timing
- Pure functions for fingerprint and path computation (unit-testable)
- Transactional semantics (temp files, atomic commit)
- Cross-platform compatibility (no bash-isms)

### Why not import generate_migration_manifest.py?

The generator script is invoked as a subprocess rather than imported
because:

1. It has its own `argparse` interface and `main()` entry point
2. Importing it would couple the upgrade script to the generator's
   internal API
3. Subprocess invocation matches how a human would run it

### Why compute fingerprint before any mutations?

The fingerprint validates that the dispositions file matches the
current Rust source tree.  If we mutated the tree first (e.g., by
checking out a different branch), the fingerprint would be wrong.
By computing it upfront, we can abort cleanly before any changes.

## Files

| File | Purpose |
|------|---------|
| `scripts/upgrade_unverified.py` | Main upgrade automation script |
| `scripts/tests/test_upgrade_unverified.py` | Unit tests |
| `scripts/upgrade_unverified.md` | This documentation |
| `verification/migration-dispositions.json` | Reviewed disposition evidence |
| `verification/migration-manifest-current.json` | Generated manifest |
| `verification/migration-manifest-current-summary.json` | Manifest summary |
