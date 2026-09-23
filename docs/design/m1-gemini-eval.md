# Evaluation: Terra Sprites — M1 "A Sprite Lives" Design

- **Evaluator:** Gemini
- **Date:** 2026-09-23
- **Target Document:** Terra Sprites — M1 "A Sprite Lives" design
- **Verdict:** **Exceptional (Ready for Implementation with Minor Clarifications)**

---

## 1. Executive Summary

The *Terra Sprites — M1 "A Sprite Lives"* design document provides an uncommonly rigorous, thoughtful, and technically mature design for a terminal artificial-life simulation inspired by Steve Grand's *Creatures*.

The document avoids the classic pitfalls of artificial-life projects—such as unconstrained evolutionary shortcuts, subjective/untested emergent behavior, race conditions in simulation loops, and floating-point desynchronization. Instead, it provides concrete specifications, mathematical invariants, determinism guarantees, and empirical behavioral acceptance tests.

---

## 2. Primary Architectural Strengths

### 2.1 The Canonical Tick as Single Source of Truth (§2.4)
Defining a strictly ordered 7-step tick lifecycle with explicit rules for when pulses latch, when actions resolve, and when decisions commit to the trace buffer eliminates an entire category of race conditions, off-by-one errors, and feedback loops:
* **Player Command Timing:** Stamping player commands for tick $t+1$ cleanly matches the visual state rendered at tick $t$.
* **Pulse Latch:** Latching event pulses (`incoming` $\to$ `live` at Step 3a) ensures pulses survive exactly one full perception-action cycle without leaking into subsequent unearned credits.
* **Emitter Sequencing:** Running emitters *after* reactions at Step 3 ensures drive reductions instantly produce reward within the same tick prior to Step 4 learning.

### 2.2 Inviolability of Physical Chemistry (§4.1, §4.3)
The core tenet:
> *"The world decides what happens to the body. The genome decides how that feels."*

By mathematically and structurally isolating physical chemicals (`energy`, `hydration`, `stamina`, `injury`, `food`, `water`) from direct genomic modification (enforced both by decode-time rejection and a 1,000-tick property test comparing random genomes against control), the design prevents evolution in M2 from trivially "optimizing away" the metabolic constraints.

### 2.3 The Fixture Ring Rule (§3.3)
Requiring all 8 neighboring tiles of a fixture to be walkable and free of fixtures is mathematically elegant:
* It guarantees that every fixture is an isolated obstacle surrounded by an open ring.
* Because fixtures cannot touch each other or border non-walkable terrain (rock/deep water), the map topology can never be pinched into disconnected sub-graphs by plant growth or player placement.
* This removes the need for expensive dynamic connectivity checks on every plant spawn or spread.

### 2.4 Empirical, Testable Behavioral Acceptance (§7.3, §7.4)
Artificial-life projects often flounder because behavior is subjective. Defining quantitative criteria with a **minimum baseline guardrail** (e.g., A1 requiring $\ge 20$ initial eats and a $\ge 50\%$ drop; A2/A3 using median of 10 seeds against an un-trained control) turns emergent learning into a deterministic CI/CD regression test.

### 2.5 Future-Proof Genome and Registry Architecture (§2.8, Appendix A)
Treating unknown genes as opaque bytes that round-trip through serde without mutation, combined with append-only integer IDs for chemicals, loci, inputs, and verbs, ensures seamless forward compatibility for M2 (reproduction/mutation) and M4 (language) without invalidating M1 saves or replays.

### 2.6 Decoupled Presentation and Semantic Seams (§6.2, §6.4)
Map rendering is completely decoupled from glyph choices via semantic tiles and CP437 palettes, allowing simple swapping to bitmap tilesets in future milestones. The `InfoPolicy` seam cleanly separates sim omniscience from UI presentation.

---

## 3. Critical Edge Cases & Subtle Ambiguities

While the document is exceptionally detailed, there are several subtle interaction dynamics that should be clarified before coding.

---

### Issue 1: Functional Overlap Between `Approach` and Interaction Verbs
* **In §5.2:** Verbs include `Approach`, `Retreat`, `Eat`, `Drink`, `Hit`, `Play`, `Rest`, `Wander`.
* **In §5.5:** The action lifecycle table states:
  > *"Eat / Drink / Hit / Play: Walks to a goal tile, then makes one attempt."*  
  > *"Approach: Walks to a goal tile. Ends on arrival."*
* **The Problem:** If `Eat`, `Drink`, `Hit`, and `Play` automatically walk the sprite to the target's goal tile and then execute, **what is the evolutionary or behavioral purpose of `Approach`?**
  * If a hungry sprite selects `Eat` on a berry bush 8 tiles away, it already approaches and eats. It never needs to learn `hunger → Approach(BerryBush)` followed by `adjacent → Eat`.
  * If `Approach` is redundant for all targets that have interaction verbs, it risks diluting the decision lobe's competition.
* **Recommendation:** Clarify the intended role:
  * *Option A (Creatures style):* Interaction verbs (`Eat`, `Drink`, `Hit`) **require adjacency**. They fail immediately with outcome `failed` if the target is not adjacent. The sprite *must* learn to `Approach` first, and once `TargetAdjacent == 1`, select `Eat`.
  * *Option B (Doc's current wording):* Interaction verbs handle transit themselves. If so, document what `Approach` is for (e.g., following other sprites without playing/hitting, or approaching objects for which the sprite has no valid interaction verb).

---

### Issue 2: Multi-Tick Action Execution vs. Trace Commitment & Snapshots
* **In §2.4 (Step 6):** *"Then each deciding sprite commits a trace entry: its snapshot plus the outcome."*
* **In §5.5:** An action like `Eat` or `Wander` can take up to 60 ticks.
* **The Questions:**
  1. **When is the trace entry committed?**
     * Does a walking sprite commit a trace entry *every tick* while walking with outcome `walking`? (If so, the 512-entry trace buffer will quickly fill with identical `walking` entries, diluting credit assignment).
     * Or does it commit a single trace entry *only on the tick the action terminates* (`applied`, `failed`, or `timed_out`)?
  2. **Which activation snapshot is credited?**
     * If a sprite decides to `Eat` at tick 0 (when `TargetDistance = 0.8` and `TargetAdjacent = 0`), takes 10 ticks to walk to the bush, and bites at tick 10 (when `TargetAdjacent = 1`):
       * If the snapshot was taken at tick 0, the concept `hungry & TargetAdjacent` was **inactive** (0), so it receives zero reinforcement!
       * If the snapshot was taken at tick 10, the decision to approach was not in that snapshot.
* **Recommendation:**
  * Clarify whether:
    1. A trace entry is committed **every tick** with the current state and step outcome, OR
    2. Snapshots occur at verb execution time (upon arrival/interaction attempt), OR
    3. Separate `Approach` from `Eat` as suggested in Issue 1, which naturally resolves this (each step of `Approach` or the final arrival is an action boundary, and `Eat` is a 1-tick adjacent action).

---

### Issue 3: "Reward Bleeding" from Continuous Digestion
* **In §4.5:**
  * Digestion converts `food → energy`.
  * Catalytic reaction: `hunger + food → food` steadily lowers hunger.
  * A **Fall** emitter on `hunger` emits `reward`.
* **The Problem:**
  Digestion takes multiple ticks. As long as `food` remains in the gut, `hunger` drops incrementally each tick.
  * If `hunger` drops continuously over 20 ticks, the Fall emitter generates positive `reward` on *all 20 ticks*.
  * If at tick $t+5$ the sprite finishes eating, and at tick $t+7$ it decides to `Hit` an adjacent sprite or eat a thornbush, that unrelated action will be reinforced by the ongoing `reward` tail from the digested berry.
* **Recommendation:**
  * Add a deadband or threshold to the Fall emitter on hunger so it only fires on large discrete drops, OR
  * Rely primarily on the **pulse** locus `ate` (which lasts exactly 1 tick) to emit the primary burst of `reward`, using hunger reduction to stop the negative drive rather than acting as a sustained reward faucet.

---

### Issue 4: Corridor Deadlocks and Action Timeout
* **In §3.7:** Sprites cannot swap tiles. When blocked for 3 ticks, they re-plan with occupied tiles made expensive.
* **In §5.5:** Actions time out after 60 ticks.
* **The Problem:** On a 1-tile-wide corridor or bridge across shallow water, two sprites meeting head-on cannot pass each other. Even with expensive occupied tiles, if no alternate path exists, the re-plan fails.
  * Under the current rules, both sprites will remain stuck for the full **60 ticks** until `timed_out`.
* **Recommendation:**
  * If a re-plan after 3 blocked ticks finds no path to the target, immediately end the action with outcome `blocked`. This frees the brain to sample a new action (e.g., `Retreat` or `Wander` away) 57 ticks sooner.

---

### Issue 5: Target Movement & Flood Cache Invalidation
* **In §3.6:** Bounded Dijkstra flood covers tiles within `sense_radius`, recomputed every 8 ticks or when entering a new tile.
* **For Sprite targets:** *"The path is re-planned whenever the target moves."*
* **The Optimization Opportunity:**
  * If target sprite $B$ moves within the existing flood basin of observer sprite $A$, sprite $A$ **does not need to re-run Dijkstra**. It only needs to look up sprite $B$'s new tile in $A$'s existing distance/predecessor grid.
  * A full re-flood should only trigger if the target leaves the sensed radius or if sprite $A$ moves. Making this distinction explicit in the spec protects the 200 ticks/sec performance target.

---

### Issue 6: Cross-Platform Floating Point & LLVM FMA Contractions
* **In §2.6:** Bit-identical results are targeted across x86_64 and aarch64 (ARM).
* **Technical Risk:** Even with IEEE-754 and pure-Rust `libm`, LLVM will opportunistically fuse `a * b + c` expressions into hardware **FMA** (Fused Multiply-Add) instructions on ARM64 and modern x86_64 with AVX2/FMA3. FMA retains infinite intermediate precision before the final rounding, producing results that differ by 1 ULP (Unit in the Last Place) from non-fused multiply and add. Over thousands of simulation ticks, this 1-ULP divergence causes total trajectory desynchronization.
* **Recommendation:** Explicitly specify in build configuration / docs:
  * Add `-C llvm-args=-fp-contract=off` to `.cargo/config.toml` (or ensure `RUSTFLAGS` disables contraction) to guarantee cross-architecture bit-identical replays.

---

## 4. Component Evaluation Matrix

| Component | Rating | Notes |
|---|---|---|
| **Ecology & World** | **9.5 / 10** | Fixture ring rule is a masterclass in elegant constraint design. Closed rule vocabulary is clean. |
| **Biochemistry** | **9.0 / 10** | Physical vs. Signal separation is robust. Watch out for reward bleeding during digestion. |
| **Brain Architecture** | **8.5 / 10** | Excellent adaptation of the Creatures 3-lobe model. Needs clarification on multi-tick trace snapshots and `Approach` vs. interaction verbs. |
| **Determinism & Saves** | **9.5 / 10** | Replay checkpoints, state hashing, and opaque gene preservation are exceptionally well planned. Watch LLVM FMA. |
| **UI & Ergonomics** | **9.5 / 10** | Ratatui + crossterm with strict CP437 support and semantic tiles decouples terminal rendering from future graphical tiles. |
| **Testing & Calibration**| **10.0 / 10** | Setting empirical baseline requirements ($\ge 20$ actions) and evaluating medians across 10 seeds sets the gold standard. |

---

## 5. Suggested Amendments to M1 Design Doc

Before writing code, consider amending the document with the following clarifications:

1. **Clarify `Approach` vs `Eat`/`Drink`/`Play`/`Hit` (§5.2, §5.5):** State whether interaction verbs require adjacency (making `Approach` a required prerequisite step), or explain why `Approach` exists if verbs walk automatically.
2. **Specify Trace Snapshot Timing (§2.4, §5.6):** Explicitly define whether trace commitment happens only when an action terminates (and whether the snapshot records the initiation state or execution state), or if every tick commits an entry.
3. **Add Early Exit on Blocked Re-plan (§3.7):** If path re-planning after 3 blocked ticks fails to find a valid route, terminate the action immediately as `blocked` rather than waiting 60 ticks.
4. **Tune Drive Drop vs Pulse Reward (§4.5):** Note that primary reward pulses should be tied to single-tick event pulses (`ate`, `drank`) to prevent lingering digestive reward tails from reinforcing subsequent actions.
5. **Cross-Platform Compiler Flags (§2.6):** Document `-C llvm-args=-fp-contract=off` in Cargo configuration to prevent FMA-induced floating point drift between x86_64 and aarch64.
