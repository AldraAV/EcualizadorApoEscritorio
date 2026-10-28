---
name: Obsidian Rack Acoustic
colors:
  surface: '#0f141b'
  surface-dim: '#0f141b'
  surface-bright: '#343941'
  surface-container-lowest: '#090f15'
  surface-container-low: '#171c23'
  surface-container: '#1b2027'
  surface-container-high: '#252a32'
  surface-container-highest: '#30353d'
  on-surface: '#dee2ec'
  on-surface-variant: '#d8c3ad'
  inverse-surface: '#dee2ec'
  inverse-on-surface: '#2c3138'
  outline: '#a08e7a'
  outline-variant: '#534434'
  surface-tint: '#ffb95f'
  primary: '#ffc174'
  on-primary: '#472a00'
  primary-container: '#f59e0b'
  on-primary-container: '#613b00'
  inverse-primary: '#855300'
  secondary: '#7bd0ff'
  on-secondary: '#00354a'
  secondary-container: '#00a6e0'
  on-secondary-container: '#00374d'
  tertiary: '#bdcee7'
  on-tertiary: '#213145'
  tertiary-container: '#a2b2cb'
  on-tertiary-container: '#35455a'
  error: '#ffb4ab'
  on-error: '#690005'
  error-container: '#93000a'
  on-error-container: '#ffdad6'
  primary-fixed: '#ffddb8'
  primary-fixed-dim: '#ffb95f'
  on-primary-fixed: '#2a1700'
  on-primary-fixed-variant: '#653e00'
  secondary-fixed: '#c4e7ff'
  secondary-fixed-dim: '#7bd0ff'
  on-secondary-fixed: '#001e2c'
  on-secondary-fixed-variant: '#004c69'
  tertiary-fixed: '#d3e4fe'
  tertiary-fixed-dim: '#b7c8e1'
  on-tertiary-fixed: '#0b1c30'
  on-tertiary-fixed-variant: '#38485d'
  background: '#0f141b'
  on-background: '#dee2ec'
  surface-variant: '#30353d'
typography:
  headline-lg:
    fontFamily: Inter
    fontSize: 24px
    fontWeight: '600'
    lineHeight: 32px
    letterSpacing: -0.015em
  headline-md:
    fontFamily: Inter
    fontSize: 18px
    fontWeight: '600'
    lineHeight: 24px
    letterSpacing: -0.01em
  headline-sm:
    fontFamily: Inter
    fontSize: 14px
    fontWeight: '600'
    lineHeight: 20px
    letterSpacing: 0em
  body-lg:
    fontFamily: Inter
    fontSize: 14px
    fontWeight: '400'
    lineHeight: 20px
  body-md:
    fontFamily: Inter
    fontSize: 13px
    fontWeight: '400'
    lineHeight: 18px
  body-sm:
    fontFamily: Inter
    fontSize: 12px
    fontWeight: '400'
    lineHeight: 16px
  label-lg:
    fontFamily: JetBrains Mono
    fontSize: 13px
    fontWeight: '500'
    lineHeight: 16px
    letterSpacing: 0.02em
  label-md:
    fontFamily: JetBrains Mono
    fontSize: 11px
    fontWeight: '500'
    lineHeight: 14px
    letterSpacing: 0.04em
  label-sm:
    fontFamily: JetBrains Mono
    fontSize: 10px
    fontWeight: '400'
    lineHeight: 12px
    letterSpacing: 0.06em
rounded:
  sm: 0.125rem
  DEFAULT: 0.25rem
  md: 0.375rem
  lg: 0.5rem
  xl: 0.75rem
  full: 9999px
spacing:
  gutter: 1rem
  gutter-sm: 0.5rem
  margin: 1.5rem
  margin-compact: 1rem
  space-xs: 0.25rem
  space-sm: 0.5rem
  space-md: 0.75rem
  space-lg: 1rem
  space-xl: 1.5rem
---

## Brand & Style

This design system targets master sound engineers, mixing specialists, and discerning desktop audiophiles who demand the tactility and precision of physical outboard studio equipment combined with modern DAW software ergonomics.

The visual direction is **Tactile Industrial Precision**. It rejects all generic sci-fi conventions, vaporwave tropes, luminous neon blooms, and artificial drop glows. The aesthetic is inspired by physical milled-chassis studio gear—black anodized aluminum faceplates, solid mechanical toggle switches, stepped rotary encoders, and physical console fader tracks. 

Surfaces favor matte obsidian and brushed slate metal textures with precise 1px micro-borders. Visual contrast is achieved strictly through deliberate surface tone stepping, crisp hair-line mechanical delineations, and disciplined status markers. The design evokes the authoritative confidence of high-end mastering hardware, where every milliradian of rotation and tenth of a decibel communicates absolute engineering rigor.

## Colors

The chromatic palette is rooted in low-reflectance studio chassis tones. It establishes an acoustic workstation environment optimized for extended sessions without visual fatigue.

* **Base Surface Stack:**
  * Canvas Root: `#0d1117` (Deep Obsidian Matte)
  * Rack Unit Surface / Containers: `#161b22` (Dark Console Slate)
  * Elevated Modules / Recessed Bays: `#21262d` (Machined Channel Bay)
  * Interactive Hover / Inset Tracks: `#282e36`
* **Structural Borders & Divisions:**
  * Subtle Chassis Border: `#30363d` (Hairline 1px borders)
  * Active Track / Division Rule: `#484f58`
* **Typography & Technical Readouts:**
  * Primary Value / Readout: `#f0f6fc` (Crisp Silver White)
  * Secondary / Unit Label: `#8b949e` (Muted Studio Grey)
  * Tertiary / Grid Coordinates: `#484f58` (Low-contrast axis calibration)
* **Functional Hardware Accents (Strictly non-luminous, zero bloom/glow):**
  * Calibration Amber (`#f59e0b`): Primary active state indicator for filter bypass, peak indicators, and active Q-node indicators.
  * Studio Cyan (`#38bdf8`): Analysis curve overlays, parametric spectrum trace, and active frequency markers.
  * Muted Slate Blue (`#64748b`): Inactive fader tracks, filter slope reference guides, and inactive band selectors.
  * Gain Reduction / Limiter Red (`#ef4444`): Crisp, un-bloomed meter warning for signal overload and hard limiting at or exceeding 0 dBFS.

## Typography

The typography couples the structural neutrality of `Inter` for channel identification and window controls with the tabular precision of `JetBrains Mono` for real-time numeric parameters.

* **Engineering Units & Values:** All decibel values (dB), frequencies (Hz, kHz), Q factors, attack/release durations (ms), and gain curves must be rendered in `JetBrains Mono` using tabular numerals (`tnum`). This eliminates horizontal jitter when dragging sliders or processing dynamic automation.
* **Control Legends:** Fader band legends (e.g., `31 Hz`, `63 Hz`, `1 kHz`, `16 kHz`) use `label-sm` in all caps or standard metric casing with generous letter-spacing to mimic silkscreened chassis labels.
* **Hierarchy Discipline:** Never use font sizes larger than 24px in core workstation views; desktop studio applications prioritize dense, high-utility operational controls over decorative headline scales.

## Layout & Spacing

Layout geometry follows a modular 19-inch rack unit paradigm, adapted for a standard 1440px desktop viewport.

* **Rack Hierarchy:** The primary canvas utilizes a fixed 1440px target width (fluid scalable between 1280px and 1920px). Layout compartments stack vertically as standard studio rack modules (1U, 2U, 3U equivalent heights) or divide horizontally into structured modular bays:
  * Top Bar: Transport, master volume, system routing, master Limiter/Clipper status.
  * Main Visualizer (Upper Tier): Spectrum display, transfer function graph, parametric EQ spline.
  * Channel Section (Lower Tier): Preamp gain staging, multi-band graphic EQ fader bay, parametric band card array, dynamic processing (Compressor).
* **Grid Architecture:** Multi-band graphic equalization layouts follow a strict 10-band, 15-band, or 31-band column strip grid where all channel strips share identical column widths, centered slider tracks, and vertically aligned indicator marks.
* **Rhythm & Density:** Studio software demands compact, predictable spacing. Vertical separation between parameter groups uses `space-md` (12px), while individual numeric readout and knob groupings use `space-xs` (4px) to preserve cognitive grouping.

## Elevation & Depth

Visual hierarchy is communicated through machined surface levels, milled recesses, and mechanical edge highlights rather than simulated atmospheric shadows or diffusion.

* **Surface Tiers:**
  * **Recessed Layer (Sub-surface):** Used for slider fader gutters, VU meter slots, and spectrum analyzer canvas backgrounds. Styled with `background-color: #0d1117`, a 1px inset border `border: 1px solid #21262d`, and an interior bevel using `box-shadow: inset 0 1px 2px rgba(0, 0, 0, 0.6)`.
  * **Base Faceplate (Ground level):** The main equipment chassis utilizes `background-color: #161b22` bounded by `border: 1px solid #30363d`.
  * **Milled Hardware Layer (Elevated level):** Fader handles, rotary knobs, push-button switches, and patch modules sit on `background-color: #21262d`. They use a crisp 1px mechanical top highlight (`box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.08)`) and a tight contact shadow (`box-shadow: 0 2px 4px rgba(0, 0, 0, 0.4)`).
* **Borders & Dividers:** Vertical boundaries between processor stages (e.g., Preamp to Graphic EQ to Compressor) are delineated by double-stroke milled channels: a 1px line of `#0d1117` directly adjacent to a 1px line of `#30363d`.
* **Zero Atmospheric Glow:** Graphs, curves, and LED indicators are rendered with 100% sharp-vector raster edges. No CSS `filter: drop-shadow()`, canvas blurs, or radiant light rings may be used on active points.

## Shapes

The shape system adopts a minimal corner treatment reflecting precision-milled aluminum hardware.

* **Corner Radii:**
  * Base tokens utilize Level 1 (`0.25rem` / `4px`). This provides crisp, near-square corners on modules, cards, and rack frames without harsh needle points.
  * Precision Elements (Fader thumb grips, segmented buttons, digital readout badges): `2px` radius.
  * Rotary Knobs & Circular Node Handles: Standard circular elements (`border-radius: 9999px`), kept strictly utilitarian without exaggerated pill geometry.
* **Fader Geometry:** Vertical fader tracks are slim 4px-wide recessed channels. Fader caps are horizontal rectangular blocks (32px wide by 14px high) with a central 1px horizontal score line for optical alignment against background scale markers.

## Components

### 1. Vertical Console Faders & Sliders
* **Track:** 4px width, recessed `#0d1117` background with 1px border `#30363d`. Features horizontal tick marks at +12 dB, +6 dB, 0 dB (unity), -6 dB, -12 dB, and -∞ dB. Unity gain (0 dB) features an extended, distinct `#8b949e` horizontal mark.
* **Cap (Thumb):** Brushed dark-metal rectangular thumb (`32px` × `14px`), colored `#21262d` with a `#484f58` border. Features a crisp center alignment score line colored `#f0f6fc` (or `#f59e0b` when active/hovered). Zero bloom.

### 2. Rotary Knobs (Parametric Gain, Preamp, Q Factor)
* **Body:** Circular dual-ring design. Outer ring: milled `#161b22` base with `#30363d` rim. Inner dial: `#21262d` with a 2px radial pointer line indicating angular position.
* **Value Arc:** Unfilled ring `#30363d`, dynamic stroke rendered in solid `#f59e0b` (Preamp/Gain) or `#38bdf8` (Frequency/Q) without glow.
* **Readout:** Directly underneath the knob, render the value in `label-md` (e.g., `1.41 Q` or `+3.2 dB`).

### 3. Hardware Push-Buttons & Band Engage Toggles
* **Base:** Rectangular `28px` height with a 2px border radius. Matte `#161b22` background with `#30363d` 1px border.
* **State Indication:** Uses an authentic miniature physical status indicator bar (2px high, 12px wide) integrated into the top edge of the button:
  * Inactive/Bypassed: `#30363d`
  * Active/Engaged: Solid `#f59e0b` (Amber) or `#38bdf8` (Cyan)
* **Labels:** `label-md` all-caps typography in `#8b949e`, shifting to `#f0f6fc` on engagement.

### 4. Parametric Frequency Graph Canvas
* **Background:** Deep `#0d1117` with hairline grid coordinates at standard octave intervals (20, 50, 100, 200, 500, 1k, 2k, 5k, 10k, 20k Hz) in `#21262d`.
* **Zero Line:** 1px dashed line `#484f58` at 0 dB.
* **Spline Trace:** Crisp 1.5px continuous vector curve in `#38bdf8`. 
* **Filter Band Control Nodes:** 8px circular handles on the curve. Inactive: solid `#161b22` with 1.5px `#38bdf8` stroke. Selected/Active: Solid `#38bdf8` center with `#f0f6fc` numeric index.

### 5. Meters (VU, Peak, Gain Reduction)
* **Style:** Segmented LED ladder or continuous micro-bar with a 1px border `#30363d`.
* **Colors:** 
  * Normal range (-∞ to -6 dB): Solid `#64748b` (Slate).
  * Nominal headroom (-6 dB to -1 dB): Solid `#f59e0b` (Amber).
  * Clip / True Peak Warning (0 dB and above): Solid `#ef4444` (Red).

### 6. Module Cards & Bay Enclosures
* **Frame:** Contained within `#161b22`, bounded by a sharp 1px border `#30363d`.
* **Header:** 32px height bar with an integrated title in `label-md` (`PREAMP`, `PARAMETRIC EQ`, `MASTER DYNAMICS`), accompanied by an inline bypass button and a 1px bottom border `#21262d`.