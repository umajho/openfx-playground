# Review: `ofx-guide-2-invert`

Overall: the crate **compiles cleanly** (`cargo check` is warning-free;
crate-local clippy reports only a style-level `doc_overindented_list_items`
warning on the crate-docs TODO at `src/lib.rs:6` — `openfx-bindings` emits its
own unrelated `missing_safety_doc` warning, out of scope here), and it is a
**faithful, spec-accurate port** of the OFX guide's Example 2 "Invert"
(`Documentation/sources/Guide/Code/Example2/invert.cpp` in the vendored openfx
tree). It inherits the post-review hardened machinery from
`ofx-guide-1-basic-machinery`:

- **No panic sites across the FFI boundary** — no `unwrap`/`expect`/`panic!`/
  `assert!` anywhere in `src/`; mutex poisoning and null pointers map to error
  codes.
- Suites are fetched in `kOfxActionLoad`, the correct place; error codes are
  mapped consistently (`kOfxStatErrMissingHostFeature`).

Deliberate improvements over the C++ original: the context check in
`DescribeInContext` (returning `kOfxStatErrUnsupported`, which the C++ skips),
no-op `CreateInstance`/`DestroyInstance` handlers for DaVinci Resolve (with an
explanatory comment), and a real `Unload` action that clears the suite state.

Issues below are ranked by severity.

---

## 1. Output image is leaked when the source image fetch fails (most important)

`src/lib.rs:411–412`:

```rust
let output_img = data.clip_get_image(output_clip, time, None)?;
let source_img = data.clip_get_image(source_clip, time, None)?;
```

If the second fetch fails, the `?` early-returns and `output_img` — already
acquired — is never released. OFX images are refcounted by the host; every
successful `clipGetImage` must be balanced by `clipReleaseImage`. The guide
itself stresses this ("it is polite to release them as soon as possible").
Ironically, the C++ original has its own bug here (the `outputImg` declared
inside the `try` shadows the outer one, so it leaks the output image on *all*
paths); the Rust port fixes the success path but regresses this error path.

**Fix:** on the `source_clip` fetch error, release `output_img` before
returning (or fetch both images through a guard that releases whatever was
acquired).

## 2. `Err(OfxStat::kOfxStatOK)` used as an abort sentinel

`src/lib.rs:341` — when the host's `abort` callback fires, `pixel_processing`
returns `Err(OfxStat::kOfxStatOK)`. This produces the right status only because
`main_entry` returns `Err` payloads verbatim (`src/lib.rs:147`). An error
variant carrying `kOfxStatOK` is a footgun: any future normalization, logging,
or `?`-propagation change in the call chain silently turns a routine
user-cancel into a real error. The C++ simply `break`s out of the loop and
returns `kOfxStatOK` normally.

**Fix:** `return Ok(())` on abort — observably identical (images are released,
the host sees `kOfxStatOK`) with no type abuse.

## 3. `#![feature(once_cell_try)]` gates the crate to nightly for no effective caching

`SharedDataHelper` (`src/shared_data_helper.rs`) holds ten `OnceLock`s, one per
suite function pointer, initialized lazily via `get_or_try_init`. But the
helper is constructed fresh on **every action call**, so the `OnceLock`s are
re-created each time — they cache nothing across calls and only spare a few
`Option` checks within a single call. The cost is real: a nightly-only feature
(`src/lib.rs:1`) plus ~90 lines of struct/constructor boilerplate.

**Fix:** resolve each function pointer at the call site with
`.ok_or(OfxStat::kOfxStatErrMissingHostFeature)?`, exactly as guide-1 does —
simpler, stable-compatible, and consistent with the sibling crate. (Unless the
helper is intended to become long-lived, in which case resolve once into a
vtable-like struct at load time.)

## 4. Error-code mapping deviates from the guide

- On image-fetch failure the C++ checks `abort()` and returns `kOfxStatOK` if
  the host is cancelling, else `kOfxStatFailed`. The port propagates the raw
  suite status (`src/lib.rs:411–412`). Arguably more precise, but a fetch
  failure caused by a host-initiated cancel is then reported as an error —
  worth recording as a deliberate choice or mirroring the guide.
- A null `kOfxImagePropData` pointer returns `kOfxStatErrBadIndex`
  (`src/lib.rs:318,330`). `BadIndex` is for property-index errors;
  `kOfxStatFailed` (what the C++ `throw` maps to) or `kOfxStatErrBadHandle`
  fits better.

## 5. Minor nits

- **Silent `continue` on impossible dst out-of-bounds** (`src/lib.rs:352`):
  the spec guarantees output bounds ⊇ render window, so `pixel_address` on the
  destination can't legitimately fail; silently skipping the row would hide a
  host bug. Returning `kOfxStatFailed` is more informative (the C++ doesn't
  check at all).
- **`y_offset * row_bytes` is `c_int` arithmetic** (`src/lib.rs:292`): overflow
  panics in debug (an abort across the FFI boundary) and wraps in release.
  Only reachable at absurd frame sizes (≳16K float RGBA), and the C++ has the
  same issue — widening to `isize` before the multiply is cheap hardening.
- **`prop_get_string` lends out `&CStr` over the host's mutable `char*`**
  (`src/shared_data_helper.rs:155–177`): asserts immutability of host-owned
  memory. Same family as the `&'static` laundering flagged in the guide-1
  review; theoretical here.
- **`tracing::error!` in `set_host` is currently a silent no-op** — the
  crate-docs TODO about initializing a tracing subscriber is still open, so a
  failed `set_host` is invisible until that lands (`src/lib.rs:3–6,112`).

## 6. Still present from the guide-1 review (declined there; listed for completeness)

- **Plugin identifier vs icon filename**: `pluginIdentifier` is
  `c"org.openeffects:InvertExamplePlugin"` (colon — faithful to the guide), but
  the bundled icon is `org.openeffects.InvertExamplePlugin.png` (dot —
  `xtask/src/learning_ofx_guide_2_build_plugin.rs:18`). The OFX packaging spec
  requires the icon to be named `pluginIdentifier + ".png"`, so hosts that
  implement the convention will never find it. (Also xtask-side and still
  unresolved: `CFBundleExecutable` is missing from the generated `Info.plist`.)
- `match true { _ if ... }` dispatch (`src/lib.rs:129`), the dead
  `#[expect(unused)] host_struct` field (`src/lib.rs:71–72`), the `&'static`
  laundering of host-owned pointers, and `set_host` double-init only logging an
  (currently invisible) error — all as in guide-1.

## 7. Things verified as correct (no action needed)

- `Describe` matches the guide exactly: label, grouping, filter-only contexts,
  float/short/byte pixel depths, `kOfxImageEffectRenderFullySafe`, host frame
  threading = 1 (`src/lib.rs:202–235`).
- `DescribeInContext`: Output then Source clips, each with RGBA/Alpha/RGB
  supported components (`src/lib.rs:251–269`).
- Render loop fidelity: abort check every 20 rows (`src/lib.rs:334`), alpha
  pass-through via `i != 3` (`src/lib.rs:363`), black fill for out-of-bounds
  source pixels (`src/lib.rs:369–374`), row-start destination pointer
  incremented across the row (the guide's guarantee that output bounds ⊇
  render window makes this sound).
- Using the output image's component count for both images is sanctioned by
  the guide (defaults guarantee source and output match in components and
  depth).
- Images fetched in `Render` are released on all post-acquisition paths except
  the one covered in issue #1.
- `Cargo.toml` (`cdylib`, workspace inheritance), `justfile`, xtask wiring, and
  `.gitignore` hygiene (`/build/`) — all consistent with the sibling crates.

---

**Bottom line:** a faithful, well-hardened port of the guide's second example.
The one thing worth fixing is the **output-image leak** (#1); #2 and #3 are
cheap robustness/simplicity wins.
