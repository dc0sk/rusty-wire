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
\eta_{\%} = 100\,\eta_{\mathrm{mismatch}}
$$

Mismatch loss in dB:

$$
L_{\mathrm{mismatch,dB}} = -10\log_{10}(\eta_{\mathrm{mismatch}})
$$

## 4) Transformer-Dependent Length Correction (Heuristic)

Rusty Wire currently applies a bounded logarithmic correction for non-1:1 transformer selections:

$$
r = \max\left(0.01,\frac{Z_t}{Z_{\mathrm{ref}}}\right),\quad Z_{\mathrm{ref}}=73\ \Omega
$$

$$
C = \operatorname{clamp}\left(1 + 0.03\log_{10}(r),\ 0.85,\ 1.15\right)
$$

$$
L_{\mathrm{corrected}} = C\,L_{\mathrm{base}}
$$

This is a practical approximation, not a substitute for NEC-based segment/current modeling.

## 5) Non-Resonant Wire Optimization

For each selected band, Rusty Wire generates resonance points from corrected quarter-wave harmonics:

$$
R = \{h\,L_{1/4,i}\mid i\in\text{bands},\ h\in\mathbb{N}\}
$$

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

For each band $i$, define resonant-point set $P_i$ in the active window. Per-band nearest distance at candidate $\ell$:

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
\text{shift}_{\%} = 100\cdot\frac{|\bar L_{1/2,\mathrm{ratio}}-\bar L_{1/2,1:1}|}{\bar L_{1/2,1:1}}
$$

$$
\text{score} = \eta_{\%} - 0.35\cdot\text{shift}_{\%}
$$

Higher score ranks earlier.

## 9) Practical Limits

Current models are intentionally lightweight and fast:
- Partial mitigation implemented: standardized antenna-height presets (7 m, 10 m, 12 m) now apply a first-order skip-distance scaling model.
- No explicit R/X feedpoint sweep vs height/ground/conductor diameter.
- No common-mode choke model or ferrite core loss/thermal derating.
- No full current-distribution solver in the optimization loop.

Current height scaling used for skip distance estimates:

$$
F_h(7\,\mathrm{m}) = 0.78,\quad F_h(10\,\mathrm{m}) = 1.00,\quad F_h(12\,\mathrm{m}) = 1.12
$$

$$
	ext{skip}_{\min,\max} = F_h\cdot \text{skip}^{\text{band-table}}_{\min,\max}
$$

For mission-critical designs, use Rusty Wire results as initial conditions and validate with NEC simulation and on-air/instrument measurements.

## References

1. ARRL Antenna Book (latest editions): practical dipole, loop, EFHW, OCFD, and inverted-V design constants.
2. RSGB Antenna Handbook: comparative geometry effects and practical construction corrections.
3. Kraus, J. D., and Marhefka, R. J., Antennas for All Applications: canonical reflection/mismatch relations.
4. Pozar, D. M., Microwave Engineering: reflection coefficient and mismatch-loss derivations.
5. ITU-R recommendations and regional band plans: operating allocations used for band tables.
