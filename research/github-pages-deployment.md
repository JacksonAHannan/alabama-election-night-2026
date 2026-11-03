# GitHub Pages deployment checks

Checked 2026-11-03 against GitHub-owned documentation, public release APIs, and the released action manifests. This note covers the requested GitHub deployment of the existing Alabama dashboard; no external settings or repository mutations were performed by the researcher.

## Hosting contract

GitHub Pages serves static HTML, CSS, and JavaScript. A project site normally lives at `https://OWNER.github.io/REPOSITORY/`, so assets, data fetches, exports, and internal links must preserve that repository path. The Axum server and SQLite collector cannot run continuously inside Pages. Run the existing collector in Actions, then publish its validated static snapshot. [GitHub Pages description](https://docs.github.com/en/pages/getting-started-with-github-pages/what-is-github-pages)

Configure the repository Pages source as GitHub Actions. A deploy job requires `pages: write`, `id-token: write`, the `github-pages` environment, and an uploaded Pages artifact. If deployment is a separate job, it must depend on the build job. Expose `steps.deployment.outputs.page_url` through the environment URL. Ordinary source checkout needs `contents: read`. [Custom workflow requirements](https://docs.github.com/en/pages/getting-started-with-github-pages/using-custom-workflows-with-github-pages), [deploy action](https://github.com/actions/deploy-pages)

Pages supports public repositories on the free plan. Published content is limited to 1 GB, deployments time out after 10 minutes, and bandwidth has a soft 100 GB monthly limit. The soft limit of 10 builds an hour does **not** apply to a custom Actions publishing workflow, so it does not prohibit the proposed five-minute publication cadence. [Pages limits](https://docs.github.com/en/pages/getting-started-with-github-pages/github-pages-limits)

## Current action releases

The documentation examples lag the live release tags. The official `GET /repos/actions/NAME/releases/latest` endpoints returned the following releases; fetching each exact tag's `action.yml` succeeded. The JavaScript actions use Node 24, supported by current GitHub-hosted Ubuntu runners.

| Action | Latest stable release observed | Publication date |
| --- | --- | --- |
| `actions/checkout` | [`v7.0.1`](https://github.com/actions/checkout/releases/tag/v7.0.1) | 2026-07-20 |
| `actions/cache` | [`v6.1.0`](https://github.com/actions/cache/releases/tag/v6.1.0) | 2026-06-26 |
| `actions/configure-pages` | [`v6.0.0`](https://github.com/actions/configure-pages/releases/tag/v6.0.0) | 2026-03-25 |
| `actions/upload-pages-artifact` | [`v5.0.0`](https://github.com/actions/upload-pages-artifact/releases/tag/v5.0.0) | 2026-04-10 |
| `actions/deploy-pages` | [`v5.0.1`](https://github.com/actions/deploy-pages/releases/tag/v5.0.1) | 2026-09-01 |

The official Git reference API resolved those exact tags directly to these commit SHAs (all `type: commit`):

```yaml
actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1
actions/cache@55cc8345863c7cc4c66a329aec7e433d2d1c52a9 # v6.1.0
actions/configure-pages@45bfe0192ca1faeb007ade9deae92b16b8254a0d # v6.0.0
actions/upload-pages-artifact@fc324d3547104276b827a68afc52ff2a11cc49c9 # v5.0.0
actions/deploy-pages@368f82528645a54fb793d4d04e342629a3f51346 # v5.0.1
```

`actions/cache/restore` and `actions/cache/save` use the same repository commit as `actions/cache`. Verification endpoint pattern: `https://api.github.com/repos/actions/NAME/git/ref/tags/VERSION`, for example [deploy-pages tag reference](https://api.github.com/repos/actions/deploy-pages/git/ref/tags/v5.0.1).

`upload-pages-artifact` packages a directory into the expected archive. Its default artifact name is `github-pages`, retention is one day, and hidden files are excluded by default. The archive must contain only files/directories, with no hard links or symbolic links. Use an explicit generated `_site` directory rather than uploading the repository or runtime directory. [Artifact action contract](https://github.com/actions/upload-pages-artifact)

## Freshness and scheduling

Five minutes is the shortest supported schedule interval. Schedules run the latest default-branch commit and only work when the workflow is present on that branch. Scheduled jobs may be delayed or dropped under load, especially at the beginning of an hour. Public-repository schedules automatically disable after 60 days without repository activity. These constraints mean the UI should say scheduled updates and display source observation times, rather than promise exact five-minute live updates. [Schedule event reference](https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows#schedule)

Implementation recommendation: offset the cron, for example `2-59/5 * * * *`, to avoid minute zero. Keep `workflow_dispatch` for recovery. Bound automatic collection to an explicit election/canvass window in UTC, documenting that a date guard skips work rather than removing the recurring cron. Cron has no year field. On November 3, 2026, Alabama is UTC−6, so election night crosses into November 4 UTC. Do not accidentally end updates at midnight UTC on election day.

Use one workflow concurrency group for collection plus publication, with `cancel-in-progress: false`, so a new scheduled run does not repeatedly interrupt collection, persistence, or deployment. GitHub permits one running workflow and normally one pending workflow per group; a newer pending invocation replaces the older pending invocation. [Concurrency reference](https://docs.github.com/en/actions/how-tos/write-workflows/choose-when-workflows-run/control-workflow-concurrency)

## Retaining valid observations across runners

Caches are immutable: saving the same exact key does not update its contents. Restore from a stable prefix and save with a unique run key, for example `alabama-returns-v1-${{ github.run_id }}-${{ github.run_attempt }}` plus restore prefix `alabama-returns-v1-`. Prefix matching selects the newest matching cache. Never use only a fixed exact key for the mutable results database. Cache misses must have a documented seed path. [Cache matching and immutability](https://docs.github.com/en/actions/reference/workflows-and-actions/dependency-caching)

Separate `actions/cache/restore` and `actions/cache/save` allow saving validated collection before the subsequent Pages deployment, so a deployment outage does not discard successfully collected observations. Save only when the collection/static-export validation step succeeds; source failures should preserve the previous valid observation and original timestamp. [Restore action](https://github.com/actions/cache/blob/main/restore/README.md), [save action](https://github.com/actions/cache/blob/main/save/README.md)

The planned checked-in `deployment/initial-results.sqlite` can seed a first run if no cache exists. It must contain only existing public election observations, preserve observation timestamps, and never overwrite a restored newer database. A 20-minute collection/build job timeout is compatible with the separate 10-minute Pages deployment limit.

Implementation recommendation: copy/checkpoint the SQLite database only after all collector connections have closed, or use SQLite's backup operation. Never copy a live WAL-mode database without its committed WAL. Keep the published `_site` allowlist distinct from cached state; the database and local runtime should not be published by accident. The cache should contain only public election data, not credentials.

Caches are not archival storage: unaccessed entries expire after seven days and the default repository storage limit is 10 GB, with older entries evicted under pressure. Unique five-minute snapshots accumulate, so cache only the compact database needed for continuity, not raw responses or the entire build/runtime directory. Preserve important final election observations separately if a permanent archive is desired. The current local database was approximately 3.1 MiB at inspection. [Cache limits](https://docs.github.com/en/actions/reference/workflows-and-actions/dependency-caching#usage-limits-and-eviction-policy)

## Acceptance checks before sharing

Implementation recommendations: test the generated site under a non-root repository prefix; confirm no browser request uses localhost or root `/api` routes; verify all exported JSON/CSV requests and source links; show retained data as stale when collection fails; ensure Refresh is a static GET rather than an unavailable server POST; check the successful Actions deployment and the actual public HTTPS page before claiming deployment is complete.
