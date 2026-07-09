---
project: rusty-wire
doc: docs/math.md
status: living
last_updated: 2026-04-30
---

# Rusty Wire Math Reference

This document defines the formulas and optimization objectives used in Rusty Wire.

Internal unit policy: all core calculations are performed in meters; feet values are derived only for imperial output display/export.

## 1) Core Length Formulas

Rusty Wire uses MHz-domain handbook formulas with velocity-factor scaling.

Base wavelength relation:

$$
\lambda = \frac{c}{f}
$$

with $f$ in Hz and $c \approx 299{,}792{,}458\ \mathrm{m/s}$.

**Velocity-factor convention (important).** The metric coefficients below are the
classic imperial handbook rules (468/936/234/1005 ft) expressed in meters. These
rules *already include* the ~0.95 bare-wire end-effect shortening relative to the
free-space half/quarter/full wavelength — e.g. $142.65/f \approx 0.9516 \cdot (c/2f)$.
Therefore $VF$ here is **not** the end-effect factor; it is an *additional*
multiplier for insulated wire and defaults to **$VF = 1.0$ (bare wire)**. Typical
insulated wire uses $VF \approx 0.90\text{–}0.95$. Do **not** set $VF = 0.95$ for
bare wire — that double-counts the end effect and yields lengths ~5 % short
(a 40 m dipole would come out ~19.1 m and resonate above the band). At $VF = 1.0$,
$142.65/7.1 \approx 20.1$ m, matching the NEC reference decks in `corpus/`.

Practical ham formulas (with $f_{\mathrm{MHz}}$ in MHz, lengths in meters):

$$
L_{1/2,\mathrm{m}} = \frac{142.65}{f_{\mathrm{MHz}}}\,VF
$$

$$
L_{\mathrm{full\ dipole},\mathrm{m}} = \frac{285.30}{f_{\mathrm{MHz}}}\,VF
$$

$$
L_{1/4,\mathrm{m}} = \frac{71.32}{f_{\mathrm{MHz}}}\,VF
$$

$$
L_{\mathrm{loop},\mathrm{m}} = \frac{306.32}{f_{\mathrm{MHz}}}\,VF
$$

Equivalent imperial constants (used internally in some code paths) are the same relations expressed as 468, 936, 234, and 1005 in feet.

Notes:
- The loop formula intentionally uses $306.32/f$ m (equivalent to $1005/f$ ft), consistent with common full-wave loop practice.
- The "full-wave dipole" value in output is still the doubled half-wave reference for dipole family guidance.

## 2) Inverted-V Geometry Adjustment

A drooping dipole typically resonates with shorter wire than a flat dipole.

Rusty Wire applies empirical shortening factors:

$$
L_{\mathrm{invV,90}} = 0.97\,L_{1/2}
$$

$$
L_{\mathrm{invV,120}} = 0.985\,L_{1/2}
$$

Leg and span relations:

$$
L_{\mathrm{leg}} = \frac{L_{\mathrm{total}}}{2}
$$

$$
\text{span}_{90^\circ} = \sqrt{2}\,L_{\mathrm{leg}}
$$

$$
\text{span}_{120^\circ} = \sqrt{3}\,L_{\mathrm{leg}}
$$

## 3) Transformer Target and Mismatch Model

For a ratio $1:n$, the target transformed impedance on the antenna side is:

$$
Z_t = 50\,n
$$

Given assumed feedpoint impedance $Z_s$, reflection magnitude is:

$$
|\Gamma| = \left|\frac{Z_t - Z_s}{Z_t + Z_s}\right|
$$

Estimated mismatch efficiency:

$$
\eta_{\mathrm{mismatch}} = 1 - |\Gamma|^2
$$

Reported efficiency percent:

$$
\eta_{\text{pct}} = 100\,\eta_{\mathrm{mismatch}}
$$

Mismatch loss in dB:

$$
L_{\mathrm{mismatch,dB}} = -10\log_{10}(\eta_{\mathrm{mismatch}})
$$

## 4) Transformer-Dependent Length Correction (Heuristic)

Rusty Wire currently applies a bounded logarithmic correction for non-1:1 transformer selections:

$$
r = \max\left(0.01,\frac{Z_t}{Z_{\mathrm{ref}}}\right)
$$

where $Z_{\mathrm{ref}}$ is the **NEC-calibrated** nominal feedpoint resistance for
the current height/ground/frequency (`nec_calibrated_dipole_r`, ~58–87 Ω over
ground, interpolated on height-in-wavelengths from nec2c solves), *not* a fixed
73 Ω. At the default 1:1 ratio this correction is a no-op, so it does not affect
default resonant lengths.

$$
C = \mathrm{clamp}\left(1 + 0.03\log_{10}(r),\ 0.85,\ 1.15\right)
$$

$$
L_{\mathrm{corrected}} = C\,L_{\mathrm{base}}
$$

This is a practical approximation, not a substitute for NEC-based segment/current modeling.

## 5) Non-Resonant Wire Optimization

For each selected band, Rusty Wire generates resonance points from the **physical
resonant quarter-wave** $L_{1/4}^{\ast} = \tfrac{71.32}{f}\,VF\cdot F_d(d)$ — the
quarter-wave with the conductor-diameter correction $F_d(d)$ of §9, but *without*
the transformer-length heuristic of §4. Resonance is a property of the wire
geometry (length, conductor diameter), not the feed, so a single shared helper
produces this point set for the non-resonant optimizer, the resonant-compromise
optimizer (§6), the OCFD split optimizer (§7), and the resonant-points shown on
screen and in exports. None of them shift when the transformer ratio changes, and
they use the identical harmonic positions. (The display, export and compromise
optimizer list only strictly in-window resonances; the non-resonant optimizer
pads its avoid-set outward — see below — so near-edge clearance stays honest.)

Each harmonic $h$ has an **impedance class** set by its parity, because the feed
sees a current maximum at odd multiples and a voltage maximum at even ones:

- **low-Z** ($h$ odd: $\lambda/4, 3\lambda/4, \dots$) — current-fed, ~35–50 Ω,
  near 50 Ω and easy for a tuner or even a direct feed;
- **high-Z** ($h$ even $=$ half-wave multiples $\lambda/2, \lambda, \dots$) —
  voltage-fed, hundreds to thousands of ohms, genuinely hard to match.

The non-resonant optimizer avoids only the **high-Z** set — the lengths a tuner
struggles with — while the desirable low-Z lengths are left available:

$$
R = \{h\,L_{1/4,i}^{\ast}\mid i\in\text{bands},\ h\in\mathbb{N},\ h\ \text{even}\}
$$

The on-screen and exported resonant-points lists show **every** resonance tagged
`low-Z`/`high-Z`, so the recommended length (which may sit near a low-Z point) is
always reconcilable against the listed points. To keep near-edge clearance
honest, the optimizer's avoid-set for each band is padded by one half-wave so the
nearest high-Z point just outside the window is still counted.

For candidate wire length $\ell$ in the configured search window:

$$
d(\ell) = \min_{r\in R}|\ell-r|
$$

Global non-resonant objective:

$$
\ell^* = \arg\max_{\ell} d(\ell)
$$

When multiple equal optima exist, the displayed recommendation is the one closest to preferred center length $\ell_c$:

$$
\ell_{\mathrm{shown}} = \arg\min_{\ell\in\mathcal{O}} |\ell-\ell_c|
$$

## 6) Resonant Compromise Optimization

For each band $i$, define resonant-point set $P_i$ (the physical resonant
quarter-wave harmonics $h\,L_{1/4,i}^{\ast}$ of §5 — conductor-corrected and
transformer-independent) in the active window. Per-band nearest distance at
candidate $\ell$:

$$
D_i(\ell) = \min_{p\in P_i}|\ell-p|
$$

Minimax objective (closest shared compromise to all selected bands):

$$
J(\ell) = \max_i D_i(\ell),\quad \ell^* = \arg\min_{\ell} J(\ell)
$$

Rusty Wire returns top local minima near the best value so users can choose practical alternatives.

## 7) OCFD Split Optimizer

For total wire length $L$, short-leg ratio $s\in[0.20,0.45]$ (1% steps):

$$
L_s = sL,\quad L_l = (1-s)L
$$

Clearance metric across bands (nearest quarter-wave harmonic clearance):

$$
C(s) = \min_i\left(\min\{\mathrm{clr}(L_s,i),\ \mathrm{clr}(L_l,i)\}\right)
$$

Objective:

$$
s^* = \arg\max_s C(s)
$$

Tie-break prefers proximity to classic one-third feed:

$$
\min |s-1/3|
$$

## 8) Advise Ranking Score

Advise mode combines mismatch and geometry-shift terms:

$$
\text{shift}_{\text{pct}} = 100\cdot\frac{|\bar L_{1/2,\mathrm{ratio}}-\bar L_{1/2,1:1}|}{\bar L_{1/2,1:1}}
$$

$$
\text{score} = \eta_{\text{pct}} - 0.35\cdot\text{shift}_{\text{pct}}
$$

Higher score ranks earlier.

## 9) Practical Limits

Current models are intentionally lightweight and fast:
- Partial mitigation implemented: standardized antenna-height presets (7 m, 10 m, 12 m) now apply a first-order skip-distance scaling model.
- Feedpoint **resistance** is estimated frequency-aware from NEC corpus anchors interpolated on height-in-wavelengths (`nec_calibrated_dipole_r`, §4); the **reactance** ($X$) is not modelled and there is no full R/X sweep vs conductor diameter.
- No common-mode choke model or ferrite core loss/thermal derating.
- No full current-distribution solver in the optimization loop.

Current height scaling used for skip distance estimates:

$$
F_h(7\,\mathrm{m}) = 0.78,\quad F_h(10\,\mathrm{m}) = 1.00,\quad F_h(12\,\mathrm{m}) = 1.12
$$

Current ground-class scaling:

$$
F_g(\mathrm{poor}) = 0.88,\quad F_g(\mathrm{average}) = 1.00,\quad F_g(\mathrm{good}) = 1.10
$$

$$
\text{skip}_{\min,\max} = F_h\cdot F_g\cdot \text{skip}^{\text{band-table}}_{\min,\max}
$$

Current conductor-diameter correction for resonant-length estimation
(metric-only input, default baseline $d_0=2.0\,\mathrm{mm}$):

$$
F_d(d) = \mathrm{clamp}\left(1 - 0.011542\ln\left(\frac{d}{d_0}\right),\,0.97,\,1.03\right),\quad d\in[1.0,4.0]\,\mathrm{mm}
$$

$$
L_{\text{corrected}} = L_{\text{impedance-corrected}}\cdot F_d(d)
$$

The template calibration CSV in [nec_conductor_reference.csv](nec-calibration.md) fits the current slope constant exactly (`k = 0.011542`) at 1.0 mm, 2.0 mm, and 4.0 mm. The runtime clamp remains intentionally broader than that observed span (`0.97 .. 1.03` instead of `0.992 .. 1.008`) until real NEC sweep data is committed.

For mission-critical designs, use Rusty Wire results as initial conditions and validate with NEC simulation and on-air/instrument measurements.

For the practical calibration workflow (data format and fitting script), see [nec-calibration.md](nec-calibration.md).

## 10) Trap Dipole Wire Budget (Estimate Only)

The trap-dipole total is a coarse whole-wire *budget* estimate:

$$
L_{\mathrm{trap},\mathrm{m}} = \frac{137.16}{f_{\mathrm{MHz}}}\,VF \quad(\equiv 450/f\ \mathrm{ft}),\qquad L_{\mathrm{leg}} = \frac{L_{\mathrm{trap}}}{2}
$$

The $450/f$ rule is a rule-of-thumb starting point, **not** a cut length. A real
trap dipole's element lengths depend on the trap inductance/capacitance and the
specific band pair, which this lightweight model does not solve. Treat the output
as an initial wire estimate and finalise element lengths against the trap
manufacturer's data or a NEC model. The **band-pair guidance** of §11 is the more
useful design output.

## 11) Trap Dipole Guidance (Per Band Pair)

When two or more bands are selected, Rusty Wire emits a per-band-pair guidance
section (upper band $f_u$, lower band $f_l$, $f_u > f_l$). For each adjacent pair:

- **Inner leg** (per side) — the upper-band element inboard of the trap, cut to a
  quarter-wave for $f_u$:
$$
L_{\mathrm{inner}} = \frac{71.58}{f_u}\,VF \quad(\approx 234.85/f_u\ \mathrm{ft})
$$
- **Total leg** (per side) — resonant on the lower band. It is ~4 % shorter than a
  plain quarter-wave because the trap's inductance electrically lengthens the
  outer section, so less physical wire reaches $f_l$ resonance:
$$
L_{\mathrm{leg}} = \frac{68.58}{f_l}\,VF \quad(\approx 225/f_l\ \mathrm{ft}),\qquad
L_{\mathrm{outer}} = \max(L_{\mathrm{leg}} - L_{\mathrm{inner}},\,0)
$$
- **Trap** — parallel-resonant at $f_u$, isolating the outer section on the upper
  band. From $f = 1/(2\pi\sqrt{LC})$ the required $L\text{–}C$ product is
$$
L[\mu\mathrm{H}]\cdot C[\mathrm{pF}] = \frac{10^6}{4\pi^2\,f_u^2} = \frac{25{,}330}{f_u^2}
$$
  and Rusty Wire lists a few practical $(C, L)$ component pairs satisfying it.

This is a first-order single-trap-per-side model; multi-trap and mutual-coupling
effects are not solved.

## References

1. ARRL Antenna Book (latest editions): practical dipole, loop, EFHW, OCFD, and inverted-V design constants.
2. RSGB Antenna Handbook: comparative geometry effects and practical construction corrections.
3. Kraus, J. D., and Marhefka, R. J., Antennas for All Applications: canonical reflection/mismatch relations.
4. Pozar, D. M., Microwave Engineering: reflection coefficient and mismatch-loss derivations.
5. ITU-R recommendations and regional band plans: operating allocations used for band tables.
