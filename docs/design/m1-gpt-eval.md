# Evaluation: Terra Sprites M1 “A Sprite Lives”

## Overall assessment

The design is strong and unusually close to implementation-ready. Its best feature is the single canonical tick order: command timing, pulse latching, learning credit and death checks all have a clear place. The data-pack and replay model, deterministic command queue, and measurable learning scenarios make the design testable rather than just aspirational.

I would resolve these cross-section issues before treating the spec as final:

1. **The training boundary still needs a washout.** In §7.3, a Play at tick 9,999 can queue a Tickle for tick 10,000. That means training can still affect the measurement window, and lingering reward can continue learning briefly after the trainer stops. Stop training early enough to drain queued commands and let reward decay before counting measurement actions.

2. **Social Play doesn’t currently relieve boredom.** The sprite verb sends `played_social`, and §4.5 uses that pulse to lower loneliness. Boredom only responds to `played`. If social Play should relieve both drives, emit `played` for both sprites too, or make the boredom pathway respond to `played_social`.

3. **Thornbush Play needs an explicit decision.** Its verb table allows Play and injects injury, though the intended thorn lesson has been Eat and Hit. Remove Play if that is accidental; otherwise state that playing with thorns is deliberately harmful and account for it in learning scenarios.

4. **Constrain spawn variation to gene values.** “Each numeric gene field” could include stable numeric references such as chemical, input or trait IDs. Specify that variation changes only designated value fields, while preserving IDs and structural fields. If reaction coefficients are mutable, preserve the physical-catalyst constraint.

I’d also make two implementation rules explicit: restrict actor/target effects such as `Inject`, `Signal` and `Push` to verb context, and state whether objects spawned during step 2 wait until the next tick before their own rules run.

With those clarifications, I’d consider the design ready to guide implementation. This evaluation reviews [the design doc](</C:/Users/Vandell/Projects/Terra Sprites/docs/design/m1-a-sprite-lives.md>); it does not change it.
