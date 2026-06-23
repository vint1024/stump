# NoirPanther fork — feature backlog & audit checklist

Master list of EVERYTHING our fork added on top of upstream Stump, derived from
the fork commit log (`git log 1ed0f99a..4ebe18a6`) + README + memory. Used as a
regression-test backlog after every upstream merge (the v0.1.5 merge silently
reverted several of these because it took `theirs` for ~97 web files + locales).

Status legend: ✅ present/verified · ⚠️ partially lost · ❌ lost/regressed · ⏳ to-audit

## A. Server / backend (Rust)
| # | Feature | Key symbols / files | Status |
|---|---------|---------------------|--------|
| A1 | Multi-folder libraries (extra paths) | `library_path` entity, migration `library_extra_paths`, `library_scan_job` multi-root loop, `extraPaths` form fields | ✅ backend OK; extraPaths form UI was orphaned — RESTORED this session (field array + picker, both forms) |
| A2 | Reversible series merging | `series_merge`+`series_merges` table, `mergeSeries`/`unmergeSeries` mutations, `fetch_map_for_library` | ✅ backend OK; UI = B10 (RESTORED) |
| A3 | Per-user content access rules (tag/genre/publisher, Unicode-folded) | `content_access_rule` entity+migration, `ContentRule*` graphql, `ContentAccessRulesEditor.tsx` | ⏳ |
| A4 | Series visibility (hide series when all books hidden) + per-user book counts | series object content-rule filtering | ⏳ |
| A5 | Metadata writeback into EPUB files (+ backup flag + cleanup) | `metadata_writeback` job, `WritebackScanGate`, librarySettings danger-zone writeback section | ⏳ |
| A6 | EPUB streaming for read-only users (read-gated /epub/{id}/file + manifest) | resource endpoints, ReadiumManifestGenerator | ⏳ |
| A7 | Server-side EPUB cover placeholder + WebP/GIF/SVG thumbnails | placeholder cover generation | ⏳ |
| A8 | Per-series thumbnail regeneration (+ regenerate-from-cover button) | `SeriesThumbnailSelector`, regenerate mutation | ⏳ |
| A9 | Offline reading w/ encryption E3 (/offline + device key + OfflineRead cap) | `device_public_keys` migration, `/offline` endpoint, `OfflineRead` permission | ⏳ |
| A10 | Book clubs at scale (cursor-paginated members/books/discussions, keyset, DataLoader) | `bookClubMembers`/`bookClubPreviousBooks`/`bookClubDiscussionsPaginated` | ⏳ |
| A11 | Unicode case-insensitive search (ulower) | `models::db::register_unicode_functions`, `ulower` in `apply_string_filter` | ⏳ |
| A12 | Search by AUTHOR (writers) — `_or` name/writers | media filter writers `_or` | ⏳ |
| A13 | Session sliding expiry (web isn't logged out every TTL) | touch_expiry throttled, OnSessionEnd cookie | ⏳ |
| A14 | Memory bounding (jemalloc + glibc arena/trim + blocking pool 128 + scanner parallelism) | main.rs MAX_BLOCKING_THREADS, Dockerfile MALLOC_* | ⏳ |
| A15 | Hide metadata field lock button without EditMetadata | gated lock icon | ⏳ |
| A16 | Single-series deletion + content-rule UX polish | per-series delete | ✅ backend OK; delete UI was orphaned — RESTORED this session (mutation + button + ConfirmationModal) |

## B. Web / UI
| # | Feature | Key symbols / files | Status |
|---|---------|---------------------|--------|
| B1 | Full Russian localization (616 keys / ~165 components → t()) | en-US/ru-RU locales + useLocaleContext in components | ✅ (recovered this session) |
| B2 | Rebrand: name, panther emblem, favicons, splash, PWA manifest | brand strings, assets | ⏳ |
| B3 | Six NoirPanther themes + Vibranium default everywhere | `themes.css` 6 classes, useApplyTheme default vibranium, ThemeSelect list | ✅ (recovered this session) |
| B4 | Vanilla Stump theme (follows OS) | useApplyTheme 'vanilla'→dark/light | ✅ (recovered) |
| B5 | 2-row filter bar on mobile/PWA (search row + controls row) | FilterHeader.tsx | ✅ (recovered this session) |
| B6 | Neon-sign login wordmark (flicker/buzz) + ambient neon wash | LoginOrClaimScene neon-sign, `.n` class | ⏳ (wordmark color fixed; verify flicker+wash) |
| B7 | Line-art panther favicon + head-only favicon | favicon assets | ⏳ |
| B8 | PWA service worker from dist root + 6 MiB precache | vite-plugin-pwa config | ⏳ |
| B9 | EPUB streaming on web (infinite-spinner fix) | reader resource endpoint usage | ⏳ |
| B10 | Series-merge UI rendered in Series settings | `MergeSeriesSection.tsx` wired into `SeriesSettingsScene` | ✅ FIXED — re-imported + rendered (ManageLibrary-gated) this session |
| B11 | Search-by-author in web | media filter writers `_or` (server) | ✅ N/A — not a bug; user searched in Series, book search matches author fine |
| B12 | Folder file-explorer icons | `Folder.png` capital (case-fix) | ✅ (fixed this session) |

## C. Apps (noirpanther — separate repo, not affected by server merge)
Tracked separately; built 0.1.1/build7 (iOS TF + Android + Mac). Not in scope of
this server-merge audit, but listed: GraphQL reading-progress migration, metadata
editing, search-by-author, offline encryption E2/E3, OPDS, book clubs, themes.

### App TODO (not yet done)
- [ ] Unified search across SERIES + BOOKS + AUTHORS in one query (spawn_task task_86802da7). Currently book search matches title+author but there's no combined cross-entity search.

---
**Audit method:** for each ⏳ item, confirm the key symbol/file exists in current
tree AND is wired/rendered (not just present-but-orphaned, like B10). After the
v0.1.5 merge the failure mode is "took theirs" → our addition reverted.
