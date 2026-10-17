# Turns when due, built early

M4's first slice, Turns when due (M4 design §10), built in M2 from the owner's patch of 2026-10-17, as the slice table allowed. It changes no world, where §10 expected the golden files to be made again. No issue: M4's slices get theirs when M3's are planned (M4 design §1.1).

## M4 "Terra"

| # | Change | Source | Sections |
|---|---|---|---|
| 1 | **Turns when due is built, in M2:** an object takes its turn at step 2 only on a tick it enters its first stage, its stage ends or one of its periodic rules (`Every`) is due, and one with none of these takes none. The schedule is worked out again when a world is loaded, so it isn't saved or hashed, and the self-check after every tick in debug builds checks it. | The owner's patch (2026-10-17) | §1.1, §10 |
| 2 | **No world changes:** the objects due still take their turns in ascending ID order, and a turn that wasn't due would have done nothing, so the world's dice fall as before. Seeds 1 to 4 end on the same state hash as on `main` after 15,000 ticks (`cargo run --release -p terra-sim --example speed -- --seed N --ticks 15000`), the determinism check (`--example determinism`) prints the same, and the golden files round-trip. On today's map it saves 1.4% of a tick's instructions on the speed check, 16.07 M to 15.85 M, counted with `valgrind --tool=callgrind` on `--example speed -- --ticks 300` less `--ticks 1`, on top of `main` at a2c212d with the lesson check of the same date; the 0.5 to 1.4 ms §10 measured is on a 4× map. | Measured on 2026-10-17 | §10 |

## M1 "A Sprite Lives"

| # | Change | Source | Sections |
|---|---|---|---|
| 1 | **An object with nothing due takes no turn:** no first stage to enter, a stage that hasn't run its full duration, and no `Every(n)` rule firing. It would have done nothing, so what step 2 does is unchanged. | M4 design §10 | §3.5.2 |
