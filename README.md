# Are we layout yet?

A static feature comparison of Rust UI layout engines, with daily benchmark history collected separately in a release archive. Initially compares Taffy, Morphorm, and Torin. Text measurement is outside the current scope.

## Run locally

Requires `just` and Node.js 22+. Stable Rust is needed only to run or export benchmarks. No npm dependencies are needed.

```sh
just serve
```

This builds the site and serves it on the loopback interface. Use `just serve 8080` to select a different port.

Open `http://localhost:8000`. The website shows the feature comparison and a benchmark results table, with a run selector for historical measurements. Fetch published data first to display numbers locally.

To fetch published measurements instead of running benchmarks, install `just` and the GitHub CLI, authenticate with `gh auth login`, then run:

```sh
just fetch-data
node scripts/build.mjs
```

The recipe uses `GITHUB_REPOSITORY` when set, otherwise the checkout's GitHub repository. Use `just fetch-data OWNER/REPO` to select a repository explicitly. It downloads and validates the release archive, replacing local `site/data/history.json`. Repeated fetches are supported.

To generate local measurements from a checkout with at least one commit:

```sh
cargo bench --manifest-path benchmarks/Cargo.toml --locked --bench pangui --features morphorm,torin -- --noplot
node scripts/collect.mjs
node scripts/build.mjs
```

Local runs append to the ignored `site/data/history.json`. Do not upload local measurements to the official archive, since they use different hardware.

## Enable publishing

1. Push this project to a GitHub repository with `main` as its default branch.
2. In **Settings → Pages → Build and deployment**, select **GitHub Actions**.
3. Ensure repository or organization policy permits workflows to write repository contents. The publishing job explicitly requests `contents: write` for release assets, not data commits.
4. Run **Daily benchmarks and website** manually, or wait for a push to `main` or the daily schedule at 04:23 UTC.

The workflow automatically creates a `benchmark-data` prerelease with a tag pointing at the publishing commit. Its single asset is `history.json.gz`. Releases and tags are created by CI, with no new data commits or contributions attributed to you. The prerelease is not marked as the latest release.

The schedule is best-effort and can be delayed by GitHub. GitHub may disable scheduled workflows in inactive public repositories.

## How the archive works

1. List releases using the authenticated GitHub API. Only a successful response with no `benchmark-data` release permits first-run initialization.
2. Download, decompress, and validate `history.json.gz`. An existing release with a missing asset, a failed request, or invalid data stops the workflow.
3. Run the vendored PanGUI benchmarks and append their Criterion mean estimates and confidence intervals. Missing Taffy scenarios or entirely missing libraries stop publication.
4. Preserve every previous run. The Actions run ID is the key, so rerunning a workflow replaces its own entry without duplicating it. Entries include the date, versions, upstream revision, compiler, CPU, repository commit, and CI link.
5. Build the website and upload old and new history as an Actions backup artifact with 90-day retention.
6. Compress and replace the release asset with `gh release upload --clobber`, then download it again and verify its bytes.
7. Deploy the static site to GitHub Pages. The JSON history is included in the site, so browsers do not need GitHub API credentials.

The release archive keeps **all runs**, not just the artifact retention window. It consumes one asset regardless of the run count. The workflow's concurrency group serializes archive updates and deployments. Do not run another writer against the same release outside that group.

GitHub asset replacement is **not atomic**. `--clobber` deletes the old asset before uploading its replacement. If that upload fails, the workflow stops and the pre-upload Actions artifact contains recovery copies. Do not delete the release to bypass a missing asset error, since that would initialize an empty history.

### Recover an interrupted upload

Use the failed workflow run's `benchmark-history-RUN_ID-ATTEMPT` artifact. It contains the updated `site/data/history.json` and, except on the first run, the previous `.history/previous-history.json.gz`.

For example, after inspecting the artifact from the intended repository:

```sh
gh run download RUN_ID --repo OWNER/REPO --name benchmark-history-RUN_ID-ATTEMPT --dir recovery
node --input-type=module -e '
import { readFileSync, writeFileSync } from "node:fs"
import { gzipSync } from "node:zlib"
writeFileSync("recovery/history.json.gz", gzipSync(readFileSync("recovery/site/data/history.json")))
'
gh release upload benchmark-data recovery/history.json.gz --repo OWNER/REPO --clobber
```

Confirm the recovered archive's run count before rerunning CI. If the initial release creation itself failed, create the prerelease with the recovered asset instead. No workflow automatically resets an existing archive.

## Benchmarks and feature data

- `benchmarks/UPSTREAM.md`: exact upstream revision, license, and local manifest adjustments
- `benchmarks/benches/pangui.rs`: upstream Criterion benchmark
- `benchmarks/pangui/`: shared workload templates
- `site/data/features.json`: manually maintained, source-linked capabilities for pinned versions
- `scripts/collect.mjs`: Criterion JSON exporter and history merge, using Cargo metadata for resolved versions
- `scripts/history.mjs`: authenticated archive download, backup, replacement, and verification
- `scripts/serve.mjs`: loopback-only static server for local development
- `.github/workflows/publish.yml`: scheduled benchmark and deployment pipeline

Dependencies are locked in `benchmarks/Cargo.lock`. Upgrades are deliberate rather than automatic nightly comparisons of moving targets. The Rust stable toolchain and hosted runner can change, so their details are recorded with each run.

Missing benchmark results mean unsupported adapter translations, not zero time. The feature table describes engine capabilities independently from benchmark participation. These timings do not verify that the engines produce equivalent geometry. See `benchmarks/UPSTREAM.md` and the upstream adapters for methodology and timing boundaries.

## Checks

```sh
cargo check --manifest-path benchmarks/Cargo.toml --locked --bench pangui --features morphorm,torin
node scripts/build.mjs
```

Pull requests run these checks without release-write permissions or deployment.
