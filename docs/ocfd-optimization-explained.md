# How OCFD Split Optimization Works

Last updated: 2026-04-10

This document explains how Rusty Wire computes the OCFD "optimized split" shown under resonant compromise results.

It is based on the current implementation used for commands such as:

```bash
rusty-wire --region 1 --mode resonant --bands 40m,20m,15m,10m --velocity 0.95 --transformer 1:4 --units both --antenna ocfd
```

## 1. What You See in the Output

For the command above, the second compromise candidate is:

```text
2. 19.20 m (62.99 ft), worst-band delta: 0.21 m (0.67 ft)
   33/67 legs: 6.40 m / 12.80 m (21.00 ft / 41.99 ft)
   20/80 legs: 3.84 m / 15.36 m (12.60 ft / 50.39 ft)
   Optimized split: 44/56 -> 8.45 m / 10.75 m (27.72 ft / 35.28 ft), worst-leg clearance: 9.76%
```

The important point is:

- `19.20 m` is the total wire length candidate.
- `44/56` is the optimized feedpoint split ratio for that total length.
- `8.45 m / 10.75 m` are the actual short and long leg lengths derived from that ratio.
- `9.76%` is the optimizer's score for that split.

## 2. The Process Has Two Separate Stages

Rusty Wire does not optimize OCFD in one single pass. It does it in two stages.

### Stage A: Find good total wire lengths

The program first computes the resonant compromise candidates.

Goal:

- Search many total wire lengths within the active window.
- For each length, compare it to the nearest resonant points of the selected bands.
- Compute the worst-band miss distance.
- Keep the best local minima and nearby practical alternatives.

This stage is still fundamentally dipole-style compromise logic.
That is why the result text says tuner-assisted and dipole-derived.

For the example above, one of those compromise totals is:

- `19.20 m`

That total length is selected before any OCFD split optimization happens.

### Stage B: Optimize the feedpoint split for that chosen total length

Once a total compromise length exists, Rusty Wire tries different short/long leg ratios for OCFD.

Goal:

- Keep both legs away from strong quarter-wave harmonic resonance points across the selected bands.
- Choose the split whose weaker leg still has the best clearance.

So the optimizer is not asking:

- "Which split gives best radiation pattern?"
- "Which split gives best feedpoint impedance match?"
- "Which split is most common in real antennas?"

Instead it asks:

- "For this fixed total wire length, which split keeps both legs furthest away from resonant trouble spots across the selected bands?"

## 3. Candidate Ratios the Code Checks

The optimizer checks short-leg ratios from 20% to 45% in 1% steps.

That means it tests:

- 20/80
- 21/79
- 22/78
- ...
- 44/56
- 45/55

For each tested short ratio $r$:

- short leg = $L \cdot r$
- long leg = $L \cdot (1-r)$

For candidate 2 where $L = 19.20$ m:

If $r = 0.44$:

$$
\text{short leg} = 19.20 \cdot 0.44 = 8.448 \text{ m}
$$

$$
\text{long leg} = 19.20 \cdot 0.56 = 10.752 \text{ m}
$$

Rounded for display:

- short leg = `8.45 m`
- long leg = `10.75 m`

That is exactly what appears in the output.

## 4. What "Worst-Leg Clearance" Means

For every tested split, the code scores both legs against every selected band.

For each band:

1. Take that band's corrected quarter-wave length.
2. For each leg, find the nearest quarter-wave harmonic.
3. Measure how far the leg is from that nearest harmonic.
4. Convert that distance into a percentage of the leg length.

The formula is:

$$
\text{clearance pct} = \frac{\text{distance to nearest harmonic}}{\text{leg length}} \cdot 100
$$

The optimizer computes this for:

- the short leg
- the long leg
- every selected band

Then it keeps only the weakest value among all of those checks.

That weakest value is the score shown as:

- `worst-leg clearance: 9.76%`

So the score answers this question:

- "Even in the worst case, how far is the more problematic leg away from a nearby quarter-wave resonance, measured as a percentage of that leg's length?"

Higher is better.

## 5. Why 44/56 Won for Candidate 2

For total length `19.20 m`, the optimizer compared all tested short-leg ratios from 20% through 45%.

For each one it computed:

- short leg length
- long leg length
- nearest-harmonic clearance of both legs against 40m, 20m, 15m, and 10m
- the worst clearance among all those checks

Then it selected the split with the highest worst-leg clearance.

For this case, `44/56` produced the best worst-case result:

- short leg = `8.45 m`
- long leg = `10.75 m`
- worst-leg clearance = `9.76%`

That means every other tested split from 20/80 through 45/55 had a worst-case leg clearance less than or equal to `9.76%`.

## 6. Why the Result Can Look Like a "Real" OCFD Ratio

You noticed that candidate 2 looks like a plausible real-world OCFD split.
That is not accidental, but it also is not based on a stored table of practical Windom ratios.

It comes from three things:

1. The search range includes realistic OCFD-style asymmetry.
2. The optimizer prefers splits that avoid obvious quarter-wave resonance conflicts in both legs.
3. If two ratios tie, the code prefers the one closer to 33/67.

That tie-break rule matters.
If two candidates are equally good by score, Rusty Wire chooses the one whose short ratio is closest to $\frac{1}{3}$.

So the result is partly:

- resonance-clearance optimization

and partly:

- a practical bias toward a common OCFD neighborhood

## 7. What the Optimizer Does Not Model Yet

This is important.

The current OCFD optimizer does **not** yet simulate:

- actual feedpoint impedance per band
- common-mode current behavior
- balun or choke effectiveness
- height above ground
- wire insulation effects beyond current velocity-factor handling
- radiation pattern or takeoff angle
- coax interaction
- real transformer losses or tuner range

So the optimized split is best understood as:

- a resonance-clearance heuristic
- for a fixed compromise total length
- useful as a practical starting point
- not a full electromagnetic design solution

## 8. How To Read the OCFD Compromise Block Correctly

When you see:

```text
2. 19.20 m ...
   33/67 legs: ...
   20/80 legs: ...
   Optimized split: 44/56 -> ...
```

Interpret it like this:

1. `19.20 m` is the candidate total wire length.
2. The next two lines show fixed reference splits for comparison.
3. The optimized line shows the best split ratio found by the current heuristic.
4. The percentage tells you how comfortably both legs avoid nearby quarter-wave harmonic trouble spots in the selected band set.

## 9. Why This Still Uses "Tuner-Assisted" Language

Even with the optimized split, the result is still labeled tuner-assisted because:

- the total length came from a shared compromise search, not a single-band exact design
- the optimizer does not yet model full feedpoint impedance behavior
- practical OCFD operation across multiple bands often still benefits from tuning and a choke/balun strategy

So the optimized split is best treated as:

- stronger than a random guess
- more informative than only 33/67 and 20/80
- still a practical heuristic, not a final guaranteed build prescription

## 10. Current Algorithm Summary

In short, the code does this:

1. Find resonant compromise total lengths.
2. For each total length, try OCFD short-leg ratios from 20% to 45%.
3. Convert each ratio into short and long leg lengths.
4. Measure each leg's distance from the nearest quarter-wave harmonic of every selected band.
5. Keep the split with the best worst-leg clearance percentage.
6. If scores tie, prefer the ratio closest to 33/67.

## 11. Practical Takeaway

For your example, candidate 2's optimized split is not magic and not hand-picked.
It is the result of a bounded search over plausible OCFD ratios, scored by worst-case resonance clearance of both legs across the selected bands.

That is why it can look like a believable real-world model:

- the search space is practical
- the scoring favors balanced multiband usability
- the tie-break nudges the result toward familiar OCFD proportions

## 12. Future Improvement Direction

A stronger next step would be to evolve this from a resonance-clearance heuristic into a more realistic OCFD design helper by adding one or more of these:

- feedpoint impedance estimation by band
- preferred transformer-ratio interaction scoring
- common-mode/choke guidance
- height-above-ground parameter effects
- split-ratio optimization constrained to known practical OCFD families

That would move the result from "useful practical starting point" toward "more build-realistic recommendation." 
