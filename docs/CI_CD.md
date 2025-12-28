# CI/CD Documentation

This document describes the continuous integration and deployment workflows for noslop.

## Overview

noslop uses a **tag-based release strategy** with three GitHub Actions workflows:

1. **CI** - Full test suite for development branches and PRs
2. **Main Branch CI** - Quick checks on main branch pushes
3. **Release** - Build and publish releases when version tags are created

## Workflows

### 1. CI Workflow (`.github/workflows/ci.yml`)

**Triggers:**
- Pushes to `develop` or `claude/**` branches
- Pull requests to `main` or `develop` branches

**Jobs (with dependencies for fail-fast):**

- **Format Check** - Ensures code follows formatting standards (`cargo fmt`)
- **Clippy Lint** - Runs Rust linter for code quality (`cargo clippy`)
- **Test Suite** - Runs all tests, builds main binary for lifecycle tests
- **Build** - Verifies release build succeeds (depends on: fmt, clippy, test)
- **Code Coverage** - Generates coverage report and uploads to Codecov (depends on: test)
- **Install Test** - Tests the installation script

**Job Dependencies:**

- Build job only runs after fmt, clippy, and test pass (fail-fast strategy)
- Coverage job runs after test suite completes
- This prevents wasted CI time when earlier checks fail

**Coverage Optimizations:**

- Excludes integration tests (lifecycle_test.rs, integration_test.rs) to prevent segfaults
- Skips doctests for faster execution
- Uses `avoid-cfg-tarpaulin` to skip dependency instrumentation
- Only measures coverage for noslop source code

**Purpose:** Comprehensive quality checks before merging code.

**Ignores:** Documentation changes (`.md`, `docs/`, etc.)

### 2. Main Branch CI (`.github/workflows/cd.yml`)

**Triggers:**
- Pushes to `main` branch

**Jobs:**
- Quick CI checks (format, clippy, tests, build)

**Purpose:** Final verification that main branch is healthy. Does NOT create releases.

**Note:** Despite the filename, this is NOT continuous deployment - it only runs CI checks.

### 3. Release Workflow (`.github/workflows/release.yml`)

**Triggers:**
- Git tags matching `v*.*.*` (e.g., `v0.1.0`, `v1.2.3`)
- Manual workflow dispatch with version input

**Versioning:** [Semantic versioning](https://semver.org/) (`MAJOR.MINOR.PATCH`)

**Process:**

1. **CI Checks** - Run full test suite before release
2. **Create Release** - Create GitHub release with installation instructions
3. **Build Binaries** - Cross-compile for all platforms:
   - Linux: `x86_64-unknown-linux-gnu`, `x86_64-unknown-linux-musl`
   - macOS: `x86_64-apple-darwin`, `aarch64-apple-darwin` (via cargo-zigbuild)
   - Windows: `x86_64-pc-windows-msvc` (via cargo-xwin)
4. **Generate Checksums** - SHA256 for all binaries
5. **Upload Assets** - Attach binaries and checksums to GitHub release
6. **Publish to crates.io** - Automatically publish (if `CARGO_REGISTRY_TOKEN` is configured)

**Cost Optimization:** All builds run on Linux runners using cross-compilation tools (cargo-zigbuild, cargo-xwin) instead of expensive macOS/Windows runners.

## Workflow Summary

| Event | Workflow | Purpose |
|-------|----------|---------|
| Push to `develop` | CI | Full quality checks |
| Push to `claude/**` | CI | Full quality checks |
| PR to `main`/`develop` | CI | Full quality checks + install test |
| Push to `main` | Main Branch CI | Quick verification |
| Tag `v*.*.*` | Release | Build binaries, create release, publish to crates.io |
| Manual dispatch | Release | Same as tag-based release |

## Creating a Release

### Prerequisites

1. Ensure all tests pass locally:
   ```bash
   make test
   ```

2. Update version in `Cargo.toml`:
   ```toml
   version = "0.2.0"
   ```

3. Commit version bump:
   ```bash
   git add Cargo.toml Cargo.lock
   git commit -m "Bump version to 0.2.0"
   git push origin main
   ```

### Option 1: Tag-based Release (Recommended)

```bash
# Create and push a version tag
git tag v0.2.0
git push origin v0.2.0
```

This triggers the release workflow automatically.

### Option 2: Manual Workflow Dispatch

1. Go to [Actions → Release](https://github.com/noslop-sh/noslop/actions/workflows/release.yml)
2. Click "Run workflow"
3. Enter version (e.g., `v0.2.0`)
4. Click "Run workflow"

### What Happens Next

1. CI checks run (format, clippy, tests)
2. GitHub release is created with tag
3. Binaries are built for all platforms
4. Binaries and checksums are uploaded to the release
5. Package is published to crates.io (if token is configured)

**Timeline:** Full release takes ~15-20 minutes to complete all builds.

## Installation Methods

Users can install noslop via:

```bash
# Quick install (latest from GitHub releases)
curl -fsSL https://raw.githubusercontent.com/noslop-sh/noslop/main/scripts/install.sh | bash

# From crates.io (after first publish)
cargo install noslop

# From source
git clone https://github.com/noslop-sh/noslop.git
cd noslop
make install

# Download binary directly
# Visit: https://github.com/noslop-sh/noslop/releases/latest
```

## Configuration

### Required Secrets

**None!** Basic releases work out of the box.

### Optional Secrets

- **`CARGO_REGISTRY_TOKEN`** - Enables automatic publishing to crates.io
  - Get token from: https://crates.io/settings/tokens
  - Required scopes: `publish-new`, `publish-update`
  - Add to: Repository Settings → Secrets → Actions
  - Used in: `release.yml` publish-crates job

### Repository Settings

**Permissions:**
- Workflows need `contents: write` permission (already configured in release.yml)
- No additional repository settings required

## Pre-commit Hooks

The repository includes a pre-commit hook ([`scripts/pre-commit.sh`](../scripts/pre-commit.sh)) that enforces:

1. Code formatting (`cargo fmt --check`)
2. Clippy lints (`cargo clippy -- -D warnings`)
3. All tests pass (`cargo test`)

**Installation:**
```bash
make install-hooks
```

This prevents committing code that would fail CI checks.

## Quality Standards

All code merged to `main` must pass:

- ✅ Rustfmt formatting
- ✅ Clippy lints with no warnings (`-D warnings`)
- ✅ All tests passing
- ✅ Successful release build
- ✅ Code coverage reporting (via Codecov)

## Branch Strategy

- **`main`** - Stable branch, protected, requires PR reviews
- **`develop`** - Integration branch for features
- **`claude/**`** - AI-assisted development branches
- **Feature branches** - Topic-specific development

**Flow:**
1. Create feature branch from `develop`
2. Open PR to `develop` (triggers CI)
3. After review and CI pass, merge to `develop`
4. When ready for release, merge `develop` to `main`
5. Tag `main` to create release

## Monitoring & Debugging

### Check Workflow Status

- **All Actions**: https://github.com/noslop-sh/noslop/actions
- **CI Runs**: https://github.com/noslop-sh/noslop/actions/workflows/ci.yml
- **Releases**: https://github.com/noslop-sh/noslop/actions/workflows/release.yml
- **Main CI**: https://github.com/noslop-sh/noslop/actions/workflows/cd.yml

### GitHub Releases

- **Latest Release**: https://github.com/noslop-sh/noslop/releases/latest
- **All Releases**: https://github.com/noslop-sh/noslop/releases

### crates.io

- **Package Page**: https://crates.io/crates/noslop
- **Version History**: https://crates.io/crates/noslop/versions

### Code Coverage

- **Codecov Dashboard**: Check CI workflow for coverage reports

### Common Issues

**Release fails at publish step:**
- Check if version in `Cargo.toml` matches the git tag
- Ensure version hasn't already been published to crates.io
- Verify `CARGO_REGISTRY_TOKEN` is configured correctly
- Note: `continue-on-error: true` prevents this from failing the entire release

**Build fails on cross-compilation:**
- Check cargo-zigbuild or cargo-xwin installation logs
- Verify target dependencies are available
- Check if any platform-specific code has issues

**Tests fail in CI but pass locally:**
- Ensure you've committed all necessary files
- Check for platform-specific assumptions
- Run `cargo test --all-features` locally to match CI

**Pre-commit hook fails:**
- Run `cargo fmt` to fix formatting
- Run `cargo clippy --fix` for auto-fixable lints
- Check test failures with `cargo test`

## Performance Optimizations

**Build Caching:**
- Uses `Swatinem/rust-cache@v2` to cache dependencies
- Separate cache keys per target platform
- Significantly reduces build times on repeated runs

**Cross-compilation:**
- All builds run on Linux (cheapest GitHub runner)
- macOS builds use cargo-zigbuild (10x cheaper than macOS runner)
- Windows builds use cargo-xwin (similar savings)

**Parallelization:**
- Multiple build jobs run concurrently
- Independent checks (fmt, clippy, test) run in parallel

## Future Improvements

Potential enhancements to consider:

- [ ] Add changelog generation on releases
- [ ] Implement automatic version bumping
- [ ] Add Docker image publishing
- [ ] Set up automatic security audits (cargo-audit)
- [ ] Add performance benchmarking in CI
- [ ] Implement nightly builds for early testing

## Resources

- [GitHub Actions Documentation](https://docs.github.com/en/actions)
- [cargo-zigbuild](https://github.com/rust-cross/cargo-zigbuild)
- [cargo-xwin](https://github.com/rust-cross/cargo-xwin)
- [Semantic Versioning](https://semver.org/)
- [crates.io Publishing Guide](https://doc.rust-lang.org/cargo/reference/publishing.html)
