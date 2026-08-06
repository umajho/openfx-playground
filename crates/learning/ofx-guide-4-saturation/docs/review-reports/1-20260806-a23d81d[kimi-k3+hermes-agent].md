# Review: `ofx-guide-4-saturation`

Overall: the crate **compiles cleanly** (`cargo clippy -p ofx-guide-4-saturation
--all-targets` and `cargo build --release` are both warning-free), and it is a
**faithful port** of the OFX guide's Example 4 "Saturation"
(`Documentation/sources/Guide/Code/Example4/saturation.cpp` in the vendored
openfx tree), extended with the general-context machinery the example exists to
demonstrate: an optional `Mask` clip defined only under
`kOfxImageEffectContextGeneral` (`src/lib.rs:236–247`), `IsIdentity`
short-circuiting at saturation 1.0 (`src/lib.rs:320–350`), and per-pixel
mask blending in the render loop (`src/processing.rs:43–52,72–78`).

It also absorbs the fixes from the earlier reviews:

- **All three images are fetched via `make_clip_image_managed`**
  (`src/lib.rs:381–404`), so `clipReleaseImage` runs on every path including
  error paths — the guide-2 review's #1 (output-image leak on a failed source
  fetch) cannot recur.
- **No panic sites across the FFI boundary** — no `unwrap`/`expect`/`panic!`/
  `assert!` anywhere in `src/`; mutex poisoning and null pointers map to error
  codes.
- Abort-on-render returns `Ok(())` (`src/processing.rs:32`), not guide-2's
  `Err(kOfxStatOK)` sentinel, and `Box::into_raw` in `CreateInstance` is
  balanced by a `Box::from_raw` drop on the `prop_set_pointer` error path
  (`src/lib.rs:296–301`) — the theoretical leak flagged for guide-3 (addendum
  §2) is handled here.
- The guide-3 `IsIdentity` regression is **not** repeated: the saturation
  comparison keeps the reference's `fabs` as `(saturation - 1.0).abs()`
  (`src/lib.rs:344`, matching `saturation.cpp:708`).

Issues below are ranked by severity.

---

## 1. `Describe` omits the general context the rest of the crate implements (most important)

`src/lib.rs:183–187`:

```rust
descriptor.prop_set_string(
    kOfxImageEffectPropSupportedContexts,
    0,
    kOfxImageEffectContextFilter,
)?;
```

Only index 0 is set. The reference sets **both** filter (index 0) and general
(index 1) (`saturation.cpp:321–329`), and the rest of this crate is built
around the general context: `DescribeInContext` accepts it
(`src/lib.rs:219–223`), defines the `Mask` clip only there
(`src/lib.rs:236–247`), and `CreateInstance` fetches the mask handle only
there (`src/lib.rs:278–282`). As written, a conforming host never learns the
general context exists, so the Mask path is dead code in practice — reachable
only from a host that describes the plugin in a context it was never told
about.

**Fix:** add a second `prop_set_string` at index 1 with
`kOfxImageEffectContextGeneral` (or record the deviation as deliberate).

## 2. Mask pixels are read at the output's component type — wrong when depths differ

`src/processing.rs:44–46`:

```rust
let mask_pix = mask_img.raw_address(x, y).map(|ptr| ptr as *mut T);
if let Some(mask_pix) = mask_pix {
    (unsafe { (into_f64)(*mask_pix) }) / (into_f64)(max)
```

The mask clip is defined Alpha-only (`src/lib.rs:240–244`), but its **pixel
depth** is independent of the output's — the OFX clip-preferences defaults do
not force an auxiliary mask clip to the output's depth. Casting mask pixels to
`T` (the output's component type) therefore misreads the mask whenever the
host delivers it at a different depth: with a `Byte` output and a `Float`
mask, this reads the low byte of each `f32` and divides by `255.0`. The
reference has the same aliasing (`saturation.cpp:552–554` uses
`mask.pixelAddress<T>(x, y)`), so this is faithful to the guide, but it is a
real latent bug that a stricter host can trigger.

**Fix:** dispatch on `mask_img.pixel_depth()` separately and normalize through
the mask's own max (or document the inherited assumption).

## 3. Minor nits

- **Redundant instance-data dance in `IsIdentity`** (`src/lib.rs:331–336`):
  the null-check + `&*ptr` deref is repeated verbatim in `DestroyInstance`,
  `IsIdentity`, and `Render` (`src/lib.rs:310–315,372–377`). A small
  `unsafe fn instance_data(&PropertySetHelper) -> OfxResult<&MyInstanceData>`
  would collapse three copies.
- **Redundant unsafe read in the blend** (`src/processing.rs:76`):
  `into_f64(unsafe { *src_pix.add(i) })` re-reads a component already loaded
  into `rgb` at `src/processing.rs:64–68` — pass `rgb[i]` instead and drop
  one `unsafe` read.
- **`blend` parameter shadows the fn name** (`src/processing.rs:111`):
  `blend(v1, v2, blend)` — rename the parameter to `amount` or `t`.
- **Mask `Option` plumbing** (`src/lib.rs:387–404`): the `match` with
  `#[expect(clippy::needless_match, clippy::manual_map)]` and the "copilot:"
  comment collapses to
  `Some(mask_clip) => data.make_clip_image_managed(mask_clip, time, None)?`
  — the `None => None` arm and both lint expects are only needed because the
  `else` is written as a match on `Option`.
- **Doc-comment typo** (`src/lib.rs:3`): "The fact **this this** is a dynamic
  library".

## 4. Still present from earlier reviews (declined there; listed for completeness)

- **Plugin identifier vs icon filename**: `pluginIdentifier` is
  `c"org.openeffects:SaturationExamplePlugin"` (colon — faithful to the guide,
  `src/lib.rs:56`), but the bundled icon is
  `org.openeffects.SaturationExamplePlugin.png` (dot —
  `build/OfxGuide4.ofx.bundle/Contents/Resources/…`). The OFX packaging spec
  requires the icon to be named `pluginIdentifier + ".png"`, so hosts
  implementing the convention will never find it. Same status as in guides
  1–3.
- **`tracing::error!` in `set_host` is still a silent no-op** — the crate-docs
  TODO about initializing a tracing subscriber remains open
  (`src/lib.rs:1–5,112–117`), inherited unchanged from guides 1–3.

## 5. Things verified as correct (no action needed)

- `Describe` otherwise matches the guide: label, grouping, float/short/byte
  depths, `kOfxImageEffectRenderFullySafe`, host frame threading = 1
  (`src/lib.rs:181–201`; only the context list in issue #1 is short).
- `DescribeInContext`: rejects contexts other than filter/general with
  `kOfxStatErrUnsupported` (`src/lib.rs:219–223`); Output then Source clips
  each with RGBA/RGB (`src/lib.rs:225–235`, matching `saturation.cpp:370–394`);
  the Mask clip Alpha-only, optional, `IsMask` (`src/lib.rs:236–247`,
  matching `saturation.cpp:396–412`); the `saturation` param with
  `kOfxParamDoubleTypeScale`, default 1.0, display min −2.0, display max 2.0,
  label and hint (`src/lib.rs:251–260`, matching `saturation.cpp:422–450`).
- Instance data lifecycle: `Box::into_raw` in `CreateInstance` /
  `Box::from_raw` in `DestroyInstance`, caching both clip handles, the
  optional mask handle, and the param handle exactly as the C++
  `MyInstanceData` does (`src/lib.rs:265–318`,
  matching `saturation.cpp:457–503`).
- `IsIdentity` protocol itself is right: fetches `saturation` at the time from
  inArgs, sets `kOfxPropName` to `Source`, returns `kOfxStatOK` /
  `kOfxStatReplyDefault` (`src/lib.rs:320–350`, matching
  `saturation.cpp:695–720`).
- Render loop fidelity: abort polled every 20 rows (`src/processing.rs:25–33`,
  matching `saturation.cpp:538`); destination addressed once per row and
  incremented (`src/processing.rs:35–38,86`, matching `saturation.cpp:541,587`);
  black fill for out-of-bounds source pixels (`src/processing.rs:88–95`,
  matching `saturation.cpp:590–596`); alpha pass-through for RGBA
  (`src/processing.rs:80–84`, matching `saturation.cpp:584–586`); per-depth
  dispatch on the output image (`src/lib.rs:415–453`, matching
  `saturation.cpp:648–676`); clamp skipped for float via
  `Clamp<T,1>` semantics (`src/processing.rs:74`, matching
  `saturation.cpp:510–513`).
- Using the output image's component count and depth for source and mask is
  sanctioned by the guide (defaults guarantee they match when no
  clip-preferences action is trapped); the mask-depth caveat is issue #2.
- `Cargo.toml` (`cdylib`, workspace inheritance), `justfile`, xtask wiring,
  and `.gitignore` hygiene — all consistent with the sibling crates.

---

**Bottom line:** a faithful, well-hardened port of the guide's fourth example,
with the earlier reviews' lessons applied. The one thing that must be fixed is
the **missing general context in `Describe`** (#1) — without it the Mask
machinery that is the point of Example 4 is unreachable. #2 is an inherited
latent bug worth fixing or documenting; #3 is cleanup.
