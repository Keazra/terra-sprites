# Terra Sprites — M3 "Body & verbs" design

- **Status:** Draft: the plan, written from the map's settled answers ([#220](https://github.com/Keazra/terra-sprites/issues/220)) and its design round ([#233](https://github.com/Keazra/terra-sprites/issues/233)). Each slice adds what its own round settles, as it's built.
- **Date:** 2026-10-26
- **History:** edited in place; each change is a file in [`changes/`](changes/)
- **Builds on:** [M1 design](m1-a-sprite-lives.md), the rulebook for everything M2 and M3 don't change, and [M2 design](m2-generations.md), whose generations, families and harsher world this keeps
- **Covers:** what milestone 3 contains, when it starts and how it reaches players, its slices and their order, content safety, what "M3 is done" means, the rules every new verb shares, and the settled rules of each piece: limbs (Slice 33), grief (34), hands (35), grab before eat (36), play as a feeling (37), grip, push and pull (38), real sleep (39), eggs in arms (40), pregnancy (41), dead bodies (42), sharing food (43), parents correcting their young (44), water play and thornbushes you can eat (45), one "take in" verb (46), the event log (47), the data checker (48), first steps (49), old worlds, and M3's acceptance (50)

---

## Decisions from planning

M3 was planned on the map [Finishing M2 and planning M3](https://github.com/Keazra/terra-sprites/issues/220), 2026-10-14 to 2026-10-19, one ticket per question, each settled with the owner in rounds of questions and closed with a resolution comment that is its record. The rounds themselves are the owner's working documents, not in this repository. [#233](https://github.com/Keazra/terra-sprites/issues/233) wrote this design from them, with one more round of its own (2026-10-26) for what the map left open. Each recommendation the owner didn't object to stands, as usual.

| # | Decision | Source | Sections |
|---|---|---|---|
| 1 | **Limbs give sprites their verbs.** Every sprite has a head, two arms and two legs. Touch, Grab, Grip, Push, Pull, Throw and Drop join the verbs; Play stops being a verb and becomes a feeling | [#164](https://github.com/Keazra/terra-sprites/issues/164), [#227](https://github.com/Keazra/terra-sprites/issues/227), [#230](https://github.com/Keazra/terra-sprites/issues/230) | §6 |
| 2 | **Limbs are hurt and heal,** and none is lost for good in M3, but everything is built for loss ([#235](https://github.com/Keazra/terra-sprites/issues/235)) | #227 Q2, Q9 | §7 |
| 3 | **A fallen berry is picked up, then eaten,** or eaten where it lies and often squished; habits are kept apart by whether the thing was in the sprite's arms; M3's rules for two-step acts | [#229](https://github.com/Keazra/terra-sprites/issues/229) | §6.3, §10 |
| 4 | **Play is boredom eased,** and each thing's data says which acts on it are play. Bored sprites try anything, the gentlest most; curiosity draws them to new things; hurting another sprite feels bad | #230 | §11 |
| 5 | **Grip, then push or pull,** with breaking free decided by the world's dice | #227 Q11, Q14–Q17 | §12 |
| 6 | **Real sleep,** a feeling of its own apart from tiredness | [#237](https://github.com/Keazra/terra-sprites/issues/237), [#288](https://github.com/Keazra/terra-sprites/issues/288) | §13 |
| 7 | **Sprites carry eggs, and a thrown egg breaks** 3 times in 4; **sprites grieve the kin they see die,** and give up on kin missed for an hour | #227 Q3, Q10; [#231](https://github.com/Keazra/terra-sprites/issues/231) Q8, Q14–Q17, Q15, Q20, Q21 | §8, §14 |
| 8 | **Families share food,** handed over by a Drop aimed at a sprite | #227 Q5, Q12, Q13 | §17 |
| 9 | **Eight more pieces from the open issues:** pregnancy, dead bodies, parents correcting their young, water play and thornbushes you can eat, one "take in" verb, an event log that says where, a checker for modders' data, and first steps; **and ten small additions** inside planned slices | [#232](https://github.com/Keazra/terra-sprites/issues/232) round 1, Q1–Q2 | §1.1, §15–§23 |
| 10 | **M3's checks are B1–B20,** with M1's and M2's again, and **M3 ends at 250 ticks a second** or more | #231, #232 round 2 | §5 |
| 11 | **Eighteen slices, in order: Slices 33–50** | #232 round 1 Q4, round 2 Q9 | §3 |
| 12 | **M3 starts once v0.2.0 is out and Looks 1–6 are in;** no M3 code merges before v0.2.0 | #232 round 1 Q5 | §2 |
| 13 | **M3 is about the sprites; the ship builds on it.** Its checks are on its own things; M4's T5 is the first to try a panel | #231 Q5, Q13 | §0, M4 design |
| 14 | **The slices' rounds run a slice or two ahead** of building, starting with Slices 33–35 | #233 round 1, Q1 | §2 |
| 15 | **The three bars held over to [#332](https://github.com/Keazra/terra-sprites/issues/332)** are judged against `main` until a look halfway through M3 | #233 round 1, Q2 | §5.4 |
| 16 | **An old world stays as it was made** | #233 round 1, Q3 | §24 |
| 17 | **Eggs that need warmth** ([#325](https://github.com/Keazra/terra-sprites/issues/325)) join pregnancy | #233 round 1, Q4 | §15 |
| 18 | **A gentle world on the title screen** ([#326](https://github.com/Keazra/terra-sprites/issues/326)) is a question for First steps | #233 round 1, Q5 | §23 |
| 19 | **The new look is the first patron beta,** and M3 reaches players in betas, v0.3.0 being the last | #233 round 1, Q6; [#344](https://github.com/Keazra/terra-sprites/issues/344) | §2 |

Rows 14 to 19 are #233's round 1, written here as recommended while the owner reads it; the round is settled once the owner says so, and this table says so then.

---

## 0. Vision

M1 made a sprite live: it feels, learns and dies. M2 made generations: sprites grow up, pair, have young, and a species changes. M3 gives them a **body**. Five limbs that can be hurt and heal decide what a sprite can do, and new verbs let it act on the world with them: touch, grab, grip, push, pull, throw and drop. A berry is picked up before it's eaten, a ball is batted and thrown for fun, and a gripped sprite struggles free. Families carry their eggs, hand food to their hungry young, correct them, sleep, and grieve the deaths they see.

The owner's principles carry over unchanged (`docs/agents/how-we-work.md`). Four matter most here:
- **The engine enforces physics only.** What a touch, a push or a pull does to a thing, and which acts on it are play, is in that thing's data (B8), so a modder makes a new toy, a bell or a lever with data alone. Each act's odds (a squish, a grip, breaking free, an egg breaking) are worked out in one place (§6.4).
- **Biases, not hard rules.** Sprites lean away from hurting their family and their eggs; nothing forbids it, and what follows teaches the rest. Content safety stays the one behavioural hard rule (§4).
- **Sprites are blank slates.** Their instincts stay as general as *hungry: eat what you're looking at*, and they learn what each thing is for, from each other as well as the player.
- **The ship builds on M3, it doesn't need it.** M3 is about the sprites. Its checks are on its own things (B8–B10), and the ship's design (M4) says it builds on them: its panels are first tried by M4's own check T5. *Need* stays the word for a sprite's drives.

**For the larger game** ([#47](https://github.com/Keazra/terra-sprites/issues/47)): limbs, hands and data-written verbs are what an NPC built on the Sprite system needs, so everything here stays in `terra-sim`'s general engine.

---

## 1. Scope

### 1.1 In M3

| Item | Slice | Issue |
|---|---|---|
| Limbs that hurt and heal, with the little picture of the sprite ([#314](https://github.com/Keazra/terra-sprites/issues/314)) | 33 | [#164](https://github.com/Keazra/terra-sprites/issues/164), [#227](https://github.com/Keazra/terra-sprites/issues/227) |
| Grief for what a sprite sees, with what it can see on the map ([#316](https://github.com/Keazra/terra-sprites/issues/316)) and deaths keeping where they happened ([#268](https://github.com/Keazra/terra-sprites/issues/268)'s world half) | 34 | [#231](https://github.com/Keazra/terra-sprites/issues/231), from [#206](https://github.com/Keazra/terra-sprites/issues/206) |
| Hands: holding by size, carrying, dropping and throwing, with how close the other choices came ([#39](https://github.com/Keazra/terra-sprites/issues/39)) | 35 | [#135](https://github.com/Keazra/terra-sprites/issues/135), #164 |
| Grab before eat | 36 | [#229](https://github.com/Keazra/terra-sprites/issues/229) |
| Play as a feeling, with novelty as a sense ([#262](https://github.com/Keazra/terra-sprites/issues/262)), the Brain tab's habit wording ([#160](https://github.com/Keazra/terra-sprites/issues/160)) and the scuff question ([#46](https://github.com/Keazra/terra-sprites/issues/46)) | 37 | [#230](https://github.com/Keazra/terra-sprites/issues/230) |
| Grip, push and pull, with fear that eases ([#72](https://github.com/Keazra/terra-sprites/issues/72)'s fear half) | 38 | #227 |
| Real sleep, with learning where to sleep ([#125](https://github.com/Keazra/terra-sprites/issues/125), [#264](https://github.com/Keazra/terra-sprites/issues/264)) | 39 | [#288](https://github.com/Keazra/terra-sprites/issues/288) |
| Eggs in arms, and eggs that break | 40 | #227, #231 |
| Pregnancy, with eggs that need warmth ([#325](https://github.com/Keazra/terra-sprites/issues/325)) | 41 | [#293](https://github.com/Keazra/terra-sprites/issues/293) |
| Dead bodies | 42 | [#85](https://github.com/Keazra/terra-sprites/issues/85) |
| Sharing food, with foster carers ([#263](https://github.com/Keazra/terra-sprites/issues/263)) | 43 | #227 |
| Parents correct their young | 44 | [#67](https://github.com/Keazra/terra-sprites/issues/67), its correcting half |
| Water play and thornbushes you can eat | 45 | [#64](https://github.com/Keazra/terra-sprites/issues/64), without juicy berries |
| One "take in" verb | 46 | [#91](https://github.com/Keazra/terra-sprites/issues/91) |
| An event log that says where | 47 | #268 |
| A checker for modders' data | 48 | [#276](https://github.com/Keazra/terra-sprites/issues/276) |
| First steps, with the gentle world's question ([#326](https://github.com/Keazra/terra-sprites/issues/326)) | 49 | [#317](https://github.com/Keazra/terra-sprites/issues/317) |
| M3 acceptance, and v0.3.0 | 50 | opened with the others |

The save converter ([#287](https://github.com/Keazra/terra-sprites/issues/287)) rides the first slice that changes a save's shape, most likely Slice 33, whose limb health a save has to hold.

### 1.2 Not in M3

Each waits for something that comes later, as #232 round 1 Q3 settled:

| Until | What waits |
|---|---|
| After M3 | Skills, practice making an act go better ([#48](https://github.com/Keazra/terra-sprites/issues/48)); ball games ([#45](https://github.com/Keazra/terra-sprites/issues/45)); choosing where to stand ([#246](https://github.com/Keazra/terra-sprites/issues/246)); rewiring chemistry genes ([#260](https://github.com/Keazra/terra-sprites/issues/260)); the young copying acts as such ([#277](https://github.com/Keazra/terra-sprites/issues/277)); learned combined ideas ([#71](https://github.com/Keazra/terra-sprites/issues/71)); hesitating, then committing ([#103](https://github.com/Keazra/terra-sprites/issues/103)); favourite things ([#65](https://github.com/Keazra/terra-sprites/issues/65)), with a parent learning its child's likes |
| Between M3 and M4 | A mod file ([#73](https://github.com/Keazra/terra-sprites/issues/73)) |
| M4 | Chasing the light ([#104](https://github.com/Keazra/terra-sprites/issues/104)); selecting any thing ([#90](https://github.com/Keazra/terra-sprites/issues/90)); area effects ([#61](https://github.com/Keazra/terra-sprites/issues/61)); a revealed map ([#134](https://github.com/Keazra/terra-sprites/issues/134)); the Cursor's powers ([#271](https://github.com/Keazra/terra-sprites/issues/271)) |
| M5 | Weight and hard hits ([#96](https://github.com/Keazra/terra-sprites/issues/96), [#43](https://github.com/Keazra/terra-sprites/issues/43)); a stress hormone ([#261](https://github.com/Keazra/terra-sprites/issues/261)); stress damaging genes ([#195](https://github.com/Keazra/terra-sprites/issues/195)); what boredom does to the body ([#133](https://github.com/Keazra/terra-sprites/issues/133)); organs ([#31](https://github.com/Keazra/terra-sprites/issues/31)); losing a limb for good ([#235](https://github.com/Keazra/terra-sprites/issues/235)); Spriggans ([#55](https://github.com/Keazra/terra-sprites/issues/55)); how far fear makes a sprite run ([#53](https://github.com/Keazra/terra-sprites/issues/53)); a memory of danger ([#266](https://github.com/Keazra/terra-sprites/issues/266)) |
| M6 | Language ([#68](https://github.com/Keazra/terra-sprites/issues/68)); family voices ([#278](https://github.com/Keazra/terra-sprites/issues/278)) |

- **Nothing new in the meadow** (#227 Q4). Berries become tiny, and the new verbs act on what's there. New toys are Wild Terra's (M5), and panels come with the ship.
- **Thrown and pushed things never hurt anyone,** as a rolling ball doesn't today (#227 Q7). The one exception is a push into something that hurts, such as thorns. Size stands in for weight.
- **Juicy berries** wait (#232 round 1 Q1): thirst is what makes childhood hard since #225, so berries that quench it would undo the unattended band.
- **The leftover halves** are filed as new issues by the slices that close their issues: juicy berries (#64, by Slice 45), a parent learning its child's likes (#67, by 44), pets and pricks meaning less each time (#72, by 38), and roughhousing as its own thing (#46's second half, by 37).

### 1.3 Alongside M3

- **Looks 7–9** (the population graph's zoom, Windows Terminal's extras, a title screen that says where you stand; [#308](https://github.com/Keazra/terra-sprites/issues/308)–[#310](https://github.com/Keazra/terra-sprites/issues/310)) run beside M3's first slices, since they touch nothing M3 changes. First steps (Slice 49) waits for Look 9.
- **The patron betas** ([#344](https://github.com/Keazra/terra-sprites/issues/344)): from v0.2.0 on, the work happens in a private repository, `Keazra/terra-sprites-dev`, and each beta reaches this repository six weeks after patrons get it. M3's issues stay here, in public, and its pull requests name them in full (`Keazra/terra-sprites#233`).
- **Upkeep** that isn't a slice goes on as before: the file split ([#286](https://github.com/Keazra/terra-sprites/issues/286)), the smaller-code findings ([#136](https://github.com/Keazra/terra-sprites/issues/136)), and the `afk` issues.

---

## 2. When M3 starts, and how it reaches players

- **No M3 code merges before v0.2.0** (#232 round 1 Q5). Slice 30 measures M2's world on `main`, and v0.2.0 ships M2 alone.
- **M3's first slice starts once v0.2.0 is out and Looks 1–6 are in,** since they change the panels, colours, help, clicking, small screens, motion and palettes M3's new screens are drawn with.
- **The slices' rounds run ahead of building** (#233 round 1 Q1). The rounds for Slices 33, 34 and 35 run first, one at a time, so the three are ready the day M3 opens. After that, each slice's round runs while the slice before it is built, so it can use what that slice measured. A question about how something looks on screen waits until Look 1's new layout is in.
- **M3 reaches players in betas** (#233 round 1 Q6, squaring #232's release plan with #344's betas):
  - **The new look is the first beta.** Looks 1–6 become 0.3.0-beta.1 for patrons, free six weeks later. This replaces #232's "last v0.2.x release carries Looks 1–6".
  - **M3's slices follow in later betas,** whenever something worth playing merges, as the owner picks. Every slice is complete and playable, so a beta can follow any of them.
  - **v0.3.0 is M3's last beta,** made once Slice 50's play-through passes. Looks 7–9 ship in whichever beta comes after they merge.
  - **A fix that can't wait** goes public at once as a patch beta, without the six weeks, as #344 has it.

---

## 3. The slices

Each slice is a GitHub issue in the "M3 — Body & verbs" milestone and its own PR, worked as in `docs/agents/how-we-work.md`'s slice loop: its open questions settled first in its own round, in plain words, then recorded in this design, a file in `docs/design/changes/` and `CONTEXT.md`. Numbering carries on from M2's, so every slice number stays unique. A slice too big for one PR ships as two (33a, 33b).

**C** critical, M medium, m minor: the playtest ranks of §5.6.

| # | Slice | Needs first | Rank | Checks |
|---|---|---|---|---|
| 33 | Limbs that hurt and heal, with the little picture | v0.2.0, Looks 1–6 | M | B1 |
| 34 | Grief for what a sprite sees, with what it can see on the map | v0.2.0, Looks 1–6 | M | Grief's tests; Slice 32's family bars |
| 35 | Hands, with how close the other choices came | 33 | M | B2 |
| 36 | Grab before eat | 35 | **C** | B3 |
| 37 | Play as a feeling, with novelty as a sense | 35 | **C** | B7, B9 |
| 38 | Grip, push and pull, with fear that eases | 33, 37 | M | B5, B8, B10 |
| 39 | Real sleep, with learning where to sleep | 33 | **C** | B12 |
| 40 | Eggs in arms, and eggs that break | 34, 35, 37 | **C** | B4 |
| 41 | Pregnancy, with eggs that need warmth | 40 | **C** | B13 |
| 42 | Dead bodies | 34, 38, 40 | M | B14 |
| 43 | Sharing food, with foster carers | 35, 36, 39, 40 | M | B6 |
| 44 | Parents correct their young | 37, 40 | M | B15 |
| 45 | Water play and thornbushes you can eat | 36, 37 | M | B16 |
| 46 | One "take in" verb | 36, 45 | M | B17 |
| 47 | An event log that says where | Look 3 | M | B18 |
| 48 | A checker for modders' data | 38 | m | B19 |
| 49 | First steps | 38, Look 9 | M | B20 |
| 50 | M3 acceptance, and v0.3.0 | all | the play-through | B11, and every check again |

- **Why this order** (#232): the body first, so every verb is built asking how usable each limb is; then hands, since eating, play, eggs and sharing all hold things; the critical slices one at a time, so only one proper playtest is ever waiting; and danger before help, since 96–100% of young in families grow up today, against M3's band of 60–90%.
- **Side by side:** 33 with 34, 38 with 39, 41 with 43, 44 with 45, 46 with 48, and 47 beside any of them once Look 3 is in. The critical slices (36, 37, 39, 40, 41) go one at a time.
- **Slice 41 is critical** since eggs that need warmth join it: an egg left to go cold is a new way to die (#233 round 1 Q4). It was medium in #232.
- **Slice 46 waits for 45,** since both rewrite water's list of what each verb does to it.
- **Every slice also keeps** M1's A9 (each new field documented with a working example, and the plain explanation Look 3 asks of every data entry), the speed rule (§5.5), and every check from the slices before it. From Slice 48 on, a slice that adds a new kind of data adds the checker's warnings for it.
- **Each new verb's words come with its slice,** and each new field is documented with its slice.

---

## 4. Content safety

The owner's two content-safety rules (`docs/agents/how-we-work.md`; M2 design §5, §18.3) hold for everything M3 adds, enforced by the engine whatever a genome or data pack says:

- **Attraction and mating attempts only ever between adults.** M3's new verbs change nothing about who is drawn to whom. Courting sprites touch each other (§11.2), and the engine's rule that the breeding drive never draws the eye to a young sprite keeps that touch on adults. G7's tests with random genomes cover every verb M3 adds: Touch, Grab, Grip, Push, Pull, Throw, Drop and handing over (§5.3).
- **Nothing about mating is ever learned by watching,** and in M3 nothing about it is ever taught by a correction either (B15).
- **Pregnancy** changes when an egg is laid, not who can mate (B13).

---

## 5. What "M3 is done" means

### 5.1 How the list works

From #231 Q1:
- **M3 has its own checks, B1 to B20** (B for Body), each written as what you'd see playing, with how it's checked.
- **The numbers are agreed before measuring,** so no slice picks its own bar. A slice sets a number only where it hangs on how its piece works, such as how long a hurt limb takes to heal, and agrees it with the owner before measuring.
- **A bar a slice can't meet goes back to the owner with the numbers,** and changes only if the owner says so.
- **M3 is done only after M2 is,** since M3's checks include M2's. Slice 50 gathers the evidence in `docs/acceptance/m3.md`, with a checklist the owner plays through.

**The runs:**

| Run | What it is | Where |
|---|---|---|
| Tests and Lab scenarios | Exact situations: A1–A3, and M3's new ones | CI and the cloud |
| The viability run | The default world left alone, seeds 1–10, 50,000 ticks (about 11 hours of play) | The cloud |
| The unattended run | 30 newborns alone, until each grows up or dies (M2 design §19.6) | The cloud |
| The lone-world run | The default world left alone, seeds 1–10, 220 hours of play (`childhood`) | The owner's PC, about a night |
| The speed check | 100 sprites | The owner's PC |

### 5.2 M3's own checks

| # | What you'd see | How it's checked, and the bar |
|---|---|---|
| **B1** | **Hurt limbs** (Slice 33). A pricked or hit sprite limps, grabs badly or crawls, and in time walks normally again | Tests for the rules: a hurt arm grabs and throws worse; hurt legs walk slowly; a sprite with both legs badly hurt crawls; a limb hurt badly enough is useless until it heals; a sprite with no usable arm can't grab, grip, push, pull, throw or drop, and eats off the ground; a hurt head makes eating and drinking harder and never kills; every limb heals. In the viability run: **seen**, a sprite with a hurt limb at half or more of the population counts, in at least 9 of 10 seeds; **not crippling**, on average at most 1 sprite in 10 has a useless limb. The slice's number: how long healing takes |
| **B2** | **Hands** (Slice 35). Sprites pick up, carry, drop and throw things on their own | In every seed of the viability run, sprites carry a berry somewhere, carry a ball in both arms, drop things and throw a ball, all on their own. How often each happens is reported |
| **B3** | **Manners** (Slice 36). Grown sprites mostly pick a fallen berry up before eating it, and the young learn it from their elders | Of grown sprites' meals of fallen berries, at least **3 in 4** were picked up first, all seeds of the viability run together. By the time they grow up, young raised among sprites pick berries up first more often than unattended young (the unattended run), in the middle seed. For information: squishes per seed, and meals eaten off the ground by sprites with no usable arm |
| **B4** | **Eggs: carried, and sometimes broken** (Slice 40). Parents carry their eggs; a due egg hatches beside the sprite holding it; a thrown egg can break, and kin who see it grieve | Tests for §14's rules. In the viability run: at least **1 egg in 10** laid by a pair is carried by a parent at some point; at most **1 in 20** is carried off by a sprite outside its family; at most **1 egg in 20** laid breaks, all seeds together. For information: who broke them (a parent, other kin, a stranger), and G5's young counted from laying as well as from hatching |
| **B5** | **Grip** (Slice 38). Sprites walk a ball along by pushing it, and grip each other; the gripped struggle free | Tests for §12's rules, and a Lab scenario: a sprite pushed into thorns learns as from the Cursor's shove (the thornbush is bad, and it comes to fear the pusher). In the viability run: in every seed, sprites grip balls and push them along, and grip each other, on their own; fewer than **1 death in 50** comes from being pushed into thorns, all seeds together. **Fear that eases** (#72): a Lab scenario where a sprite fears another that then stays near it for two hours of play without hurting it; by the end its fear is at most **half** what it was |
| **B6** | **Sharing** (Slice 43). Parents hand food to their hungry young and partners | In the viability run: at least **one hand-over within a family per hour** of play in the middle seed; at least **3 in 4** hand-overs go to the giver's own young or partner, which also keeps fostering (#263) rare in the starter species; G5's bar on adults dying of hunger or thirst holds (§5.4). Tests: both feel it, and a hungry giver's pull goes quiet. **The 90% cap** on young raised among sprites stays: if sharing pushes past it, the starter species' pull to share is turned down. For information: how many young were fostered |
| **B7** | **Play** (Slice 37). Bored sprites still play, with Play no longer a verb | In the viability run, play per sprite per hour is at least **three quarters** of M2's Play at M2's end. More is welcome. It counts play with balls and with sprites together, and expects the mix to change |
| **B8** | **Things say what verbs do** (Slice 38). A modder's new thing does what its data says when it's touched, pushed or pulled | A test loads a test data pack with a made-up thing written only in data (a bell a touch rings, or a lever a pull moves), and each verb does what its data says. No code in the game names it |
| **B9** | **Curiosity** (Slice 37). Sprites try a new kind of thing on their own | A Lab scenario: the default world with its balls taken out, one ball by the wall, no player. In at least **9 of 10 seeds,** a sprite touches it within **2 hours of play**. Measured on `main` at 20fe1ef, before Slice 29: within about an hour in 9 of 10 seeds |
| **B10** | **Teaching Push and Pull** (Slice 38). You reward a sprite for pushing a ball, and it pushes balls more; the same for pulling | A2's way: a trainer pets each push for about two hours, then two hours are measured without it, against the same seed untrained. The trained sprite pushes at least **1.5 times** as often as the untrained one, and at least **20 times** in the two hours measured. The same for pulling. Playable in Story mode |
| **B11** | **Played** (Slice 50). The owner plays M3 in Windows Terminal and ticks a checklist | Story mode on the default world. The checklist is the critical pieces' lines plus a few medium ones, written in full by the acceptance slice, and the owner finishes every first-steps world (B20) |
| **B12** | **Sleep** (Slice 39). Sprites lie down for long sleeps a few times a day, wake rested, and notice less while asleep | Tests for the rules: sleepiness builds while awake, whatever the sprite does; only sleep eases it; sleep costs a fifth less than standing awake; a sleeping sprite notices only what's close or loud. In the viability run, grown sprites sleep in long stretches in every seed (how long a stretch is, the slice agrees). G5's bar on grown sprites dying of hunger or thirst and Slice 32's family bars hold (§5.4). The sleep round also settles how sleep counts against [#318](https://github.com/Keazra/terra-sprites/issues/318)'s bar on grown sprites being on the move |
| **B13** | **Pregnancy** (Slice 41). After mating, a mother carries her egg inside her for a while, hungrier and slower, and the Body tab says so; then she lays it beside her | Tests: a mating that works starts a pregnancy, not an egg; she lays it when it ends; she can't conceive again until she has laid; if she dies pregnant, the egg dies with her, with its own cause of death in the data, and kin who see it grieve both. G7's tests hold as they are. In the viability run: eggs laid per hour at least **three quarters** of Slice 40's on the same seeds; at most **1 pregnancy in 20** ends with the mother dying; G5's three bands and Slice 32's family bars hold, an egg's time in sight of a parent counted from laying. The slice's numbers: how long a pregnancy lasts, and, with warmth (§15.2), how long an egg can be left |
| **B14** | **Dead bodies** (Slice 42). A dead sprite leaves a body that fades after a while; click it to see who it was and how it died; a searching family that finds it grieves at once; sprites can grip and drag it | Tests: every death leaves a body, as the species' data says, an object type a modder can change or leave out; a selected sprite that dies leaves the selection on its body; the inspector shows its name, age, generation, family and cause of death; a sprite missing that one which sees the body grieves at once and stops looking; a body can be gripped, pushed and pulled but isn't food in its data; no sense takes a body for a living sprite. In the viability run: in every seed a sprite looking for missing kin finds the body at least once, on its own; on average at most **1 body for every 10 living sprites**. For information: how each search ended. The slice's number: how long a body lasts |
| **B15** | **Parents correct their young** (Slice 44). A parent near its young steps in when the young tries something the parent has learned is bad; the young stops, and learns from it as from the Cursor's Correct | Tests: correcting is a leaning in the genes, not a rule; a sprite corrects only a try it judges bad itself; the young learns it as its own lesson, as from the Cursor's Correct, not as one learned by watching; a correction never injures; nothing about mating is ever taught by a correction. A Lab scenario, A1's way: a young sprite among thornbushes beside a parent that has learned they're bad, against the same seeds with correcting off: over two hours of play the corrected young is pricked at most **half as often**, seeds 1–10 together. In the viability run: parents correct their young in every seed, on their own; at least **3 in 4** corrections are by the young's own parent; Slice 32's "a young sprite has a parent in sight half its childhood" holds; G5's band for young raised among sprites holds, **90% at most** |
| **B16** | **Water play and thornbushes you can eat** (Slice 45). Bored sprites splash in water; a starving sprite may eat a thornbush | Tests: both written only in data, with no code naming either (B8's way); playing with water counts as play; eating a thornbush eases some hunger and pricks. A1 and the thorn trap pass as they are. In the viability run: water play in every seed, counted in B7; thornbush meals at most **1 meal in 50**, and at least **3 in 4** of them by a sprite with hunger above **0.75**; G5's three bands hold, the unattended 10–30% included, since water play draws the young to water |
| **B17** | **One "take in" verb** (Slice 46). "Went to drink the water", never "went to eat the water" | Tests: Eat and Drink retire and Take in joins, the old numbers kept and never reused (M1 design §1.4); each thing's data says what taking it in is called and does; every M1 and M2 save opens, its Eat and Drink read as Take in (A6); a seed run for an hour of play shows no line naming a verb its thing doesn't have. In the viability run: G5's hunger-and-thirst bar, B3's manners and A1–A3 hold; fruitless tries at taking something in, per sprite per hour, are no more than fruitless eats and drinks together in Slice 45's world |
| **B18** | **An event log that says where** (Slice 47). Each line says where; a click takes the view there, flashes the tile and selects a living sprite; `j` opens the whole log | Snapshot tests at 100 columns and Look 4's smaller sizes, in every theme; a click centres the view and flashes the tile; a death's place shows; `j` is in the help and the manual's key table; filters work in the full view. The bar: the full log keeps at least **the last hour of play** of a full default world, middle seed, and with it open the frame stays within budget at Max with 100 sprites (the debug overlay shows it) |
| **B19** | **A checker for modders' data** (Slice 48). `--check-data <folder>` warns about data that loads but can never do anything, naming the file and entry | For each kind of warning on #276's list, plus any the slice's round adds for M3's data, a test pack with just that fault gives just that warning; it never refuses data; the checks are in `terra-sim`; the reference docs show the flag with an example. The bar: **no warnings** on the game's own data or on any example in `docs/reference/`, and under a second on the game's own data on the owner's PC |
| **B20** | **First steps** (Slice 49). From the title screen, three or four small worlds, each with one goal on the status line; finishing one offers the next; the last teaches pushing a ball | Each world and goal are data; a test plays each with a scripted Cursor that rewards each right try and finishes it, and the same world with nothing done doesn't finish on its own. The bar: the scripted player finishes every world within **30 minutes of play** (under 8 minutes at 4×) in at least **9 of 10 seeds**. That's far quicker than B10's two hours of training, so the push world is built to learn fast: a young sprite, one ball beside it, little else to do |

- **Planting** isn't a check of its own: a carried berry dropped on grass can already sprout (1 time in 10), and B2 checks that sprites carry and drop things. Beds are M4's.
- **The small additions** need no checks of their own beyond the ones named above: they're covered by tests and snapshot tests in their slices. A scuff in gentle play, if the play round brings it in, keeps B1's "at most 1 sprite in 10 has a useless limb". How varied sprites' favourite play is (#65) is reported for information.
- **M3 is where the cap on families first bites.** Slice 29 dropped M2's top bar on young raised among sprites (for M2, "at least 60%"), and M3 keeps #231's 60–90%. So the slices that make childhood easier (sharing, correcting, water play) each check they stay under 90%.

### 5.3 M1's and M2's checks in M3

| # | The check | In M3 |
|---|---|---|
| A1 | A sprite learns to keep away from thornbushes | **Holds,** counting any contact with a thornbush: Take in (Eat, until Slice 46), Hit, Touch, Grip and any other new verb that reaches one, since Play is gone |
| A2 | Petted for playing with balls, a sprite plays with them 1.5 times as much | **Holds,** with play as §11 defines it: any act that sets the ball moving (a touch, a hit, a throw, or a push or pull of one gripped) |
| A3 | Shocked for hitting, sprites hit half as much | **Holds as it is** |
| A4 | Enough sprites live | Folded into G5 by [#223](https://github.com/Keazra/terra-sprites/issues/223) |
| A5 | Speed | §5.5 |
| A6 | Replays match, saves load, and old saves open | **Holds.** A save from M2's end joins M1's golden saves, and every M1 and M2 save opens; old Play reads as Touch (§6.5), and from Slice 46 old Eat and Drink as Take in |
| A7 | A million ticks with a random player: no crash | **Holds.** The world's self-check learns M3's rules: a thing in a sprite's arm is on no tile, and a grip's two ends stand side by side |
| A8 | It all works in Windows Terminal | **Holds,** with M3's lines in B11's checklist |
| A9 | Every data file is documented with a working example | **Holds,** for every new field: limbs, sizes, what Touch, Push and Pull do to a thing, the squish odds, the grip odds, sharing, an egg's break chance and cause of death, and each later slice's fields |
| G1 | Left alone for 220 hours, at least 9 of 10 seeds last | **Holds,** run again on the owner's PC on M3's finished world |
| G2 | The family tree | **Holds** (tests) |
| G3 | The population graph | **Holds** (tests) |
| G4 | The species changes over the generations | **Holds** as [#226](https://github.com/Keazra/terra-sprites/issues/226) leaves it, read from the same 220-hour run, with the race against the starter's genes as its second test (M2 design §19.6) |
| G5 | Unattended newborns: 10–30% grow up. Young raised among sprites: 60–90%. Adults dying of hunger or thirst: under 1 in 20 of grown deaths | **Holds, the same three bands,** on M3's world, the adults' bar as §5.4 says |
| G6 | Fast-forward | **Holds** (tests) |
| G7 | A child is never a mating target | **Holds.** Its tests with random genomes cover every verb M3 adds (§4) |
| — | Slice 32's families (M2 design §15.9): lone eggs at most 1 in 50; at least 9 pair eggs in 10 come after a dance; partners in sight half the time; an egg has a parent in sight three quarters of its time; a young sprite has a parent in sight half its childhood | **Hold,** counted over the first 200,000 ticks of the same 220-hour run. They also catch the cost of searching families (§8). Partners in sight is one of §5.4's three |

**For information in M3's long run** (#225, noted on #233): left alone for 100 hours, 92% of seed 5's sprites had lost the gene that sends a parent looking for its young, and parents without it raised 1.27 times as many grown young, since nothing in M2 threatens the young. The long run reports whether the gene comes back once M3's dangers, and eggs that need warmth, give it a use.

### 5.4 The three bars held over to #332

Three of M2's bars were held over to [Revisit harshness once more features are in](https://github.com/Keazra/terra-sprites/issues/332) by the owner's calls of 2026-10-20 and 2026-10-24 (M2 design §10.9, §19.6, §19.8): grown sprites dying of hunger or thirst (under 1 in 20 of grown deaths: 5.8% in the viability run since room nearby, mostly while a world fills), bites tried while too full (at most 1 in 10, missed in one seed of the long run), and partners in each other's sight (at least half the time: 44%). M3's checks say all three hold, so as written every slice would fail them before it had changed anything. Until #332 is settled (#233 round 1 Q2):

- **Each slice does no worse than `main`** on these three, allowing a tenth for noise, as the thornbushes' move into the bushes did ([#77](https://github.com/Keazra/terra-sprites/issues/77)). A slice that does worse goes back to the owner with the numbers before it merges.
- **A look halfway, once Slices 39 and 43 are in** (real sleep, and parents sharing food, which change how grown sprites rest and eat): the 220-hour run on the owner's PC, with #332's bars, is #332's revisit. If they still miss, the numbers go to the owner, and a fix goes into a slice before M3's acceptance.
- **At M3's acceptance,** the three hold as G5 and Slice 32's bars ask, unless the owner has changed them by then.

### 5.5 Speed

From #231 Q4:
- **The floor holds:** 100 sprites at 200 ticks a second or more on the owner's PC.
- **M3 ends at 250 or more** on the owner's PC (312 when #231 settled it), so M4's bigger ship starts with room to spare.
- **A slice that slows a tick by more than a tenth** looks into why before it merges, as #210 did for Slice 32.
- **The brain grows as M3 needs;** the limit is the speed, not the brain's size. No speed slice is planned: one is added before acceptance only if M3 falls short of 250.

### 5.6 How much each piece is playtested

From #231 Q12, Q18, Q19, as `docs/agents/how-we-work.md` has it: **critical**, the owner plays it before the slice merges, a proper session with a checklist written for it, its lines joining M3's play-through; **medium**, a short look, 10 to 15 minutes with 3 to 5 lines to check; **minor**, no playtest, tests, runs and snapshots being enough. A slice takes the highest rank of what's in it.

| Piece | Rank | Why |
|---|---|---|
| Grab before eat, squishes and manners | **Critical** | It changes most meals (fallen berries are 57–85% of them), and how a sprite looks when it eats |
| Play as a feeling | **Critical** | Play is everywhere on screen. If it feels wrong, sprites look dull |
| Real sleep | **Critical** | Long sleeps change much of what's on screen, and a sleeping young sprite is a new way to die |
| Eggs in arms, breaking, grief | **Critical** | A new way to die and a new sadness, and only playing shows whether it's a moment or a slaughter |
| Eggs that need warmth | **Critical** | A new way to die, for eggs left alone |
| Hands: holding by size, carrying, dropping, throwing | Medium | Seen all through the critical playtests anyway |
| Hurt limbs | Medium | Limping and crawling must read clearly on the map and the Body tab |
| Grip: push, pull, breaking free | Medium | New and visible, but its rules are exact and tests pin them down |
| Pregnancy, dead bodies, sharing, correcting, water play, take in, the event log, first steps | Medium | A moment to see once; the runs count how often |
| Curiosity, and teaching Push and Pull | Medium | Teaching is tried once |
| Things saying what verbs do; the data checker; words for the new verbs; docs for the new data; speed | Minor | Tests prove it, and nothing new shows on screen that the critical playtests don't show anyway |

---

## 6. The body and its verbs

What every new verb shares. Each piece's own rules are in its section.

### 6.1 The verbs

- **The verbs in M3** (#164, #227, #230): Approach, Retreat, Wander, Rest, Eat, Drink, Hit, Mate, BreedAlone, Call and Court, as in M2; and new, **Touch, Grab, Grip, Push, Pull, Throw and Drop.** Slice 46 replaces Eat and Drink with **Take in** (§20). Speak stays reserved for M6.
- **Play stops being a verb** (§11). Its number is retired and never used again, as verb numbers are append-only (M1 design §1.4).
- **Each verb works on anything whose data says what it does** (#227 Q1, B8): Touch, Push and Pull on any thing, so a panel, a button or a lever needs no new code; Grab, carrying and Drop on anything small enough to hold.
- **Handing over is a Drop aimed at a sprite** (#227 Q5): no new verb (§17).

### 6.2 What a sprite's limbs allow

- **Hard and soft gates** (#164): Grab, Grip, Push, Pull, Throw and Drop aren't offered at all with **no usable arm**. Everything else is softened, not barred: a hurt arm grabs and throws worse, a sprite hits with its arms or its legs, and hurt legs walk slowly or crawl. One arm is enough unless the thing's size needs two (§9.1).
- **Each limb is tracked on its own,** and the verbs, the Body tab and the brain's senses all ask how usable it is (#227 Q9), so a later milestone's limb loss (#235) or organs (#31) need no new questions of them.

### 6.3 Acts in two steps

M3's rules for an act that takes two choices, such as picking a berry up and then eating it, or gripping a ball and then pushing it (#229):

1. **The sprite feels the state between the two steps:** something in its arms, or something gripped.
2. **An act that can't be done at all without the first step,** such as throwing with empty arms, isn't a choice until it can be.
3. **An act that can be done without the first step but goes worse,** such as eating a fallen berry off the ground, is a choice, and habits about it are kept apart by that state. "Eating berries off the ground" and "eating berries in its arms" are two habits.
4. **Each step has something drawing the sprite to it:** an instinct, or what it has learned about the thing.

This applies to the Cursor's teaching and to what's learned by watching as well as to a sprite's own habits. Which instincts each act gets is its own slice's call.

### 6.4 Each act's odds, in one place

Every chance an act has (a berry squished when eaten off the ground, a grip taken, breaking free, an egg breaking where it lands) is read from the thing's data and worked out by one part of the engine, never inside each act. So skills ([#48](https://github.com/Keazra/terra-sprites/issues/48)), when they come, change how an act goes by changing that one part, and the world's single random source (M1 design §2.3) keeps replays exact.

### 6.5 Old genomes, saves and mods that say Play

From #230 Q7:
- **Anything that says Play loads, with Play read as Touch,** and is written back saying Touch: genome files, saves (habits by verb, actions under way, the world's own copy of the data), mods' objects and tags, and Lab scenarios' trainers.
- So an M2 world opens in M3 with its sprites' lessons carried over ("playing with thornbushes is bad" becomes "touching thornbushes is bad"), an exported M2 genome imports, and an M2 mod's toy is still a toy.
- **Replays don't change:** one plays only in the build that recorded it.
- Slice 46 does the same for Eat and Drink, read as Take in (§20).

### 6.6 On screen

- **An act chosen out of boredom reads as play** ("Going to play with the ball") and ends with what the sprite did ("Batted the ball", "Kicked the ball", "Threw the ball"). Touching a sprite reads "Played with Pip". The ship's touches read as touches ("Touched the panel") (#230 Q8).
- **The event log keeps listing every act that was play.**
- **A fruitless try** reads "Tried to eat the water, but nothing happened", not "…, but it was empty", which was written for a bush with no fruit (#91's wording fix, with M3's words for the new verbs).
- **The exact words are each slice's,** with its verb.
- **The Body tab is full** (about 23 lines in 21 rows at 100×30), so the little picture (§7.3) shows the limbs and what each arm holds without taking rows.

---

## 7. Limbs that hurt and heal (Slice 33)

From #164, #227 Q2 and Q9, and #231's B1.

### 7.1 The body

- **Every sprite has five limbs:** a head, a left and a right arm, and a left and a right leg. Every sprite is born with all five, and genes don't change them.
- **A hit, a prick or a crash can hurt a limb.** A hurt arm grabs and throws badly; hurt legs walk slowly, and a sprite with both legs badly hurt crawls.
- **A limb hurt badly enough is useless until it heals.** A sprite with no usable arm can't grab, grip, push, pull, throw or drop, and eats off the ground, squishes and all (§10).
- **The head is tracked like the others.** A hurt head makes eating and drinking harder, but it never kills in M3. Death by injury stays as it is (M1 design §4.10).
- **Limbs heal with time, and none is lost for good.** Limb loss is designed later ([#235](https://github.com/Keazra/terra-sprites/issues/235)), but each limb is tracked so it can join (§6.2).
- **Limb health is saved with the sprite,** so Slice 33 is most likely the first slice that changes a save's shape, and carries the save converter ([#287](https://github.com/Keazra/terra-sprites/issues/287)): the old schema's types frozen as they are, and the old-save fixups moved into the migration.

### 7.2 What the player sees

- A hurt sprite limps, crawls or fumbles on the map, as its slice settles, and the Body tab says which limbs are hurt and how badly.

### 7.3 The little picture ([#314](https://github.com/Keazra/terra-sprites/issues/314))

- A small figure of the selected sprite on the Body tab, about 8 by 4 cells in half blocks (`▀ ▄ █`, all CP437): head, arms and legs shaded by how hurt each is, and what's in each arm (from Slice 35).
- Its colours come through the theme's palette, so themes still decide every colour, and the tile front end can later draw real art behind the same slots.
- It grows with the sprite's life stage.

### 7.4 Left for its round

- How long healing takes (B1's number), and whether rest or sleep speeds it.
- What hurts which limb: a hit, a prick, a crash, a fall from a throw.
- How hurt is how clumsy: the soft gates' numbers, and how slow a crawl is.
- How the map shows a limp or a crawl, in every theme and the ASCII look.
- What the little picture shows beyond the limbs, since there's no body colour gene, and where it sits once Look 1's layout is in.

---

## 8. Grief for what a sprite sees (Slice 34)

From #231 Q15, Q20 and Q21, which brought part of [#206](https://github.com/Keazra/terra-sprites/issues/206) into M3.

### 8.1 Grief

- **A sprite grieves any of its kin it sees die:** its parents, its young, its brothers and sisters and its partner (from Slice 40, an egg breaking too). The engine gives the sense; the genes say what it feels: in the starter, the `grief` hormone, as for a lost partner today, so evolution can change it.
- **No sprite knows of a death it didn't see.** The owner: "Sprites are not telepathic!" Family who die out of sight are missed as if only away: eggs, young, parents and partners.
- **After an hour out of sight in a row** (`breeding.apart`, the hour that frees partners today), the sprite gives up on that one: it stops missing it, and the worry turns into grief. It can't tell the dead from the lost, so a living one away that long is given up on too; if it turns up, it's found family again, and partners stay freed, as today.
- **This changes Slice 32** (M2 design §15.4, §15.6): today a partner who dies anywhere frees the survivor at once with a `lost_partner` pulse, and the family senses drop a young one, a parent or an egg the moment it's gone.
- **Searching:** a sprite remembers where it last saw each one it misses, and goes back there to look, as parents already do for their eggs.

### 8.2 Where deaths happened ([#268](https://github.com/Keazra/terra-sprites/issues/268)'s world half)

- **A death keeps where it happened,** in the record of the dead. Grief needs it, and dead bodies (§16), mourning places (#206) and the event log (§21) use it.

### 8.3 What the sprite can see, on the map ([#316](https://github.com/Keazra/terra-sprites/issues/316))

- **A key tints the tiles the selected sprite's senses reach,** so the player sees why one parent grieves and another only worries. Tests check the tinted tiles are exactly those its senses reach, and from Slice 39 a sleeping sprite's are fewer.

### 8.4 Left for its round

- Whether the tint is the senses' real reach (the perception flood, which walls shape) or a ring, and how it sits beside the remembered places and lineage colours on the map.
- The key, and what the Body tab says of a sprite that's missing someone or has given up.
- How strongly the starter genes grieve each kind of kin.

The rest of the owner's note stays on #206: a shortlist of who matters, a lasting memory whose pain fades, and mourning places.

---

## 9. Hands (Slice 35)

From #164, #227, #229 and #230.

### 9.1 Holding by size

| Size | Held |
|---|---|
| **Tiny** (new; berries become tiny) | In one arm, stacking up to the handful |
| **Small** (an egg) | In one arm, one at a time |
| **Medium** (the ball) | In both arms |
| **Large** | Not held |

- **Holding is a state, not a verb:** what's in a sprite's arms after a Grab. Moving while holding is carrying. In plain words on screen, a thing is **in its arms**. The Cursor's Hold stays the Cursor's.
- **A thing in a sprite's arms is on no tile,** and the world's self-check holds it to that (A7).
- **A sprite feels whether it has something in its arms** (#229). Food in the arms feels like nothing: only eating eases hunger, so sprites don't carry food about without eating it (the raccoon's trap in #228's research).
- **Picking something up with full arms lets go of what's in the way,** which falls at the sprite's feet. A berry joins berries already in an arm, up to the handful.

### 9.2 Dropping and throwing

- **Drop** puts what's in its arms on a free tile beside it; aimed at a sprite, it's a hand-over (§17).
- **Throw is aimed at what the sprite is looking at** (#230 Q12), so "throw the ball to Pip" is one choice. A sprite throws less far than the Cursor, and a thrown thing never hurts what it hits (§1.2).
- **A berry that rots in a sprite's arm falls where the sprite stands,** and may sprout like any rotting berry: 1 in 10, where there's room (#229).
- **Planting:** a carried berry dropped on grass can sprout, the same 1 in 10. Beds that always grow it are M4's.

### 9.3 How close the other choices came ([#39](https://github.com/Keazra/terra-sprites/issues/39))

- **The Brain tab adds the runners-up** under the chosen verb, with their scores, and greys the verbs the sprite couldn't choose, with why: "no free arm", "nothing to throw".

### 9.4 Left for its round

- The handful: how many berries one grab takes, and one arm holds.
- What a sprite senses about its arms, such as whether the thing it's looking at is the one in its arm.
- How far a throw goes, and what a thrown thing does when it lands short of what it was aimed at.
- The instincts for Grab, Drop and Throw, and how strong each is. Bored sprites' pull to pick up and throw is Slice 37's (§11.2).
- How a sprite carrying something shows on the map.

---

## 10. Grab before eat (Slice 36)

From #229, settled 2026-10-15, with #164's rule that fruit on a bush and water are taken directly.

- **There are two ways to eat a fallen berry.**
  - **Picked up, then eaten from the arms:** two choices, each with an instinct as general as the starter species' others: *hungry: grab what you're looking at*, and *hungry, with something in its arms: eat it*. Today's *hungry: eat what you're looking at* stays.
  - **Eaten where it lies,** which often **squishes** the berry. The owner: "Picking up is delicate and gentle; smashing your face into something to eat it is hard to do. It should fail fairly often, so that picking up naturally ends up the better choice when Sprites learn it. And hey, they can start teaching each other good manners!" It starts at 1 in 2, set in each food's data, so a modder's food can be easier or harder to eat off the ground, or impossible.
- **A squished berry** is destroyed, the sprite isn't fed, and the berry rots at once where it lay, so it may sprout like any rotting berry.
- **Habits are kept apart by whether the thing was in the sprite's arms** (§6.3), so a squish teaches "eating a berry off the ground squishes it", not "eating a berry from my arms doesn't work". Picking up wins as sprites learn, and the young take the manners on by watching. Worth isn't split: a berry eases hunger either way.
- **Fruit on a bush and water** are taken directly, as today.
- **A sprite with no usable arm eats off the ground,** squishes and all, so a sprite that one day loses both arms can still eat.
- **Eggs and hungry sprites** (#227's caveat): *hungry: grab what you're looking at* means newborns will often pick eggs up until they learn eggs aren't food. An egg isn't food in its data, so a sprite that tries to eat one learns it doesn't work. That's what an egg is, not a ban.
- **Why it matters, measured:** fallen berries are most of what sprites eat, 57–71% of meals in a world's first 50 hours on the owner's PC (`main` at a5fd7ad), rising to 78–85% where genes changed. Fallen berries that rot uneaten are also how new bushes grew before Slice 29's seedlings, so this touches how the bushes renew.
- **Skills stay later.** The owner's end goal is skills, "basically memories but for the body, that decay very slowly", and "habit might be the better word for it than Skill". So what M3 builds is habits; practice making an act go better (fewer squishes) comes later, through §6.4.

**Left for its round:** how strong each instinct is; whether the squish chance should differ by life stage; B3's information lines.

---

## 11. Play as a feeling (Slice 37)

From #230, settled 2026-10-17, with #262 and #160 joining from #232.

### 11.1 What play is

- **Play is a feeling, not a verb:** boredom eased by something a sprite does. Nothing in the game's code knows what a toy is. Each thing's data says which acts on it are play, by sending the `played` feeling, so a modder makes a new toy with data alone.
- **The built-in ball:** any act that sets it moving is play. A touch bats it a little way, a hit kicks it further, a throw tosses it, and pushing or pulling one it grips moves it a tile. Picking it up or putting it down isn't play, so sprites don't carry balls about for nothing.
- **Everything else is as today.** Sprites are the only other playmates. A thornbush hurts whatever is done to it, touches and grips included. Touching anything else does nothing, so a bored sprite that touches a bush learns "touching bushes doesn't work". A berry a bored sprite picks up and tosses isn't a toy: it neither eases boredom nor disappoints, and a tossed berry that rots may sprout where it landed.
- **An egg is never a toy:** throwing one never counts as play, by its data (§14).

### 11.2 What a bored sprite does

The owner: "A bored Sprite will try anything, up to and including hitting, kicking, throwing, grappling, and should learn what is/isn't acceptable by way of how others react to it. They should probably bias gentler actions over harder actions."

- **A ladder of leanings, gentlest strongest.** The starter genes give boredom a pull towards each act on what the sprite is looking at: **touch** most, then **pick up and throw** (*bored, with something in its arms: throw it*), then **grip, push and pull**, and **hit** least. Each is as general as *hungry: eat what you're looking at*.
- **Other sprites.** Hitting or gripping another sprite is a little fun for the one doing it; the other is hurt by a hit and struggles against a grip (§12). What usually follows teaches the rest: the other hits back or runs, and empathy (§11.5) makes it feel bad either way.
- **Family is spared by a leaning, not a rule.** The starter genes lean strongly against the hard acts (hit, throw, grip) on a sprite's own family (its young, its partner, its parents) and their eggs, as they already lean against hitting family. A sprite can still do it, and learns from what follows. Gentle acts on family, a touch, picking up and putting down, are fine. Strangers and their eggs get no special leaning. The owner: "if we want to discourage a behaviour, we should do it with biases as opposed to hard rules." If the eggs slice breaks more eggs than B4 allows, the fallback is a leaning against the hard acts on every egg, still a leaning.
- **Lonely and courting sprites touch.** *Lonely: touch what you're looking at* and *aroused, looking at a stranger of the other sex: touch it* take the place of the other two Play instincts, so courting pairs still get to know each other by playing (M2 design §15.2).

### 11.3 Curiosity, and novelty as a sense ([#262](https://github.com/Keazra/terra-sprites/issues/262))

- **Curiosity:** *looking at something new to it: touch it.* Sprites sense how new the thing they're looking at is (how unfamiliar its kind is), and the instinct quietens as the thing becomes familiar, and in a sprite hurt lately. A touch out of curiosity that does nothing costs nothing. It's how sprites touch new things unprompted, which the ship's first doors build on.
- **Novelty is a sense genes can read,** not only something inside the brain, so evolution can write a sensation seeker (novelty dulling pain, or raising curiosity), and a modder's new thing is new to every sprite at first. A test: a genome that reads it touches new things more often than one that doesn't, on the same seed.

### 11.4 Learning to play

- **Play teaches how a sprite played, when it was bored.** The starter genes turn a fall in boredom into a little reward, so whatever act eased it becomes a good habit for that kind of thing: "kicking balls is good". Each sprite finds its own ways to play, and the young pick up their parents' style by watching. A pick-up just before a throw shares a little of the throw's reward, so sprites can come to pick balls up in order to throw them. It rewards play only when the sprite was bored.
  - **The risk:** reward also makes a sprite like the thing a little in general, so sprites may linger by balls, and a crowd at a ball can start grown sprites' crowding fights. The slice measures it; if it can't fix it, it comes back to the owner with the numbers, with "only what with" (no reward) as the fallback.
- **Nothing in the genes points a sprite at toys.** It learns them as it learns food and water, and the young learn sooner by watching.

### 11.5 Empathy

- **Hurting another sprite feels bad.** When a sprite's own act hurts another sprite (a hit, a push into thorns, an egg it breaks), it feels a new feeling, *I hurt someone*, which the starter genes turn into a little punishment. So it learns that act is bad even when the other runs.
- **No pang when hitting back** at a sprite that has just hurt it, or one it's afraid of. Sprites still stand up for themselves, and the lesson lands on picking fights.
- **How strongly a sprite feels it is a gene,** so sprites differ, and evolution can change it. Spriggans ([#55](https://github.com/Keazra/terra-sprites/issues/55)) "will notably lack or have severely reduced empathy".
- It also works on the deliberate mistake: crowded grown sprites that hit a neighbour now learn not to, by themselves as well as from the player.
- **The slice watches** that the pang, which also makes the one it hurt seem a little bad to the sprite for a while, doesn't drive friends apart; and that sprites that feel little don't win too many fights.

### 11.6 Company and boredom

- **Touching another sprite is social play:** both feel it, they get to know each other, and it eases loneliness as today. It eases boredom about as much as today (a touch 0.15, a ball 0.5), a little less at most. The owner: "Playing with other Sprites shouldn't be incredibly nerfed, just not as fun as a toy."
- **A ball that reaches another sprite** is social play for both: a ball a sprite kicked, threw or pushed that reaches another eases loneliness and lets them get to know each other, and eases boredom a little more on top of the kick's own play. The one it reaches feels less of it than the one that sent it (the owner: "Joy for both, but less for the one being hit"). It's the ball's data that says so, so a modder's toy can do the same. Playing with another sprite with a ball becomes the most fun there is, and the start of ball games ([#45](https://github.com/Keazra/terra-sprites/issues/45)).
- **Kicks and pushes go straight on,** away from the side the sprite walked up to, as today; a kick reaches another sprite by luck. Choosing where to stand waits ([#246](https://github.com/Keazra/terra-sprites/issues/246)).
- The slice reports the share of play that's with a ball (1% before M3). If it stays that low, more balls in the default world (a preset number, nothing new in the meadow) is the next thing to try.

### 11.7 The Brain tab ([#160](https://github.com/Keazra/terra-sprites/issues/160))

- The Brain tab's habit lines are rewritten anyway when Play becomes Touch and habits split by whether the thing was in the sprite's arms, so #160's leftover wording goes with them: each habit line says its verdict in words ("won't touch").

### 11.8 Left for its round

- How far a touch, a kick and a throw send a ball, and how much each eases boredom.
- How strong each rung of the ladder is, how strong the leaning against hurting family is, how small the reward for play is, and how much company eases boredom.
- How big the empathy pang is, the starter species' empathy gene, and how "just hurt me" and "afraid of" are read for hitting back.
- How much less a ball's social play is for the one it reaches.
- The thornbush's rules for the new verbs.
- What novelty measures (a kind never met, a sprite it doesn't know, a place it hasn't been), and whether the starter genes read it.
- **Whether gentle play carries a tiny chance of a scuff** ([#46](https://github.com/Keazra/terra-sprites/issues/46)), keeping B1's "at most 1 sprite in 10 has a useless limb" if so.
- The words on screen. For information, the slice reports how varied sprites' favourite play is ([#65](https://github.com/Keazra/terra-sprites/issues/65)).

---

## 12. Grip, push and pull (Slice 38)

From #227 Q11 and Q14–Q17, the owner's idea in place of a one-tile shove, with #72's fear half from #232. The first steps towards grappling.

### 12.1 Gripping

- **Grip is a verb of its own:** a sprite takes hold of a thing without lifting it, so it can grip what it can't Grab, such as another sprite. A grip's two ends stand side by side, and the world's self-check holds them to that (A7).
- **While it has a grip, a push** moves the thing one tile straight away from the sprite, and the sprite steps into the tile it left. **A pull** is the reverse: the sprite steps straight back, and the thing follows into its old tile. Directions are straight lines in M3; steering a push waits for skills (#48).
- **A fixture never moves:** gripping a lever or a button and pushing or pulling it does what its data says (B8).
- **Gripping, then pushing or pulling, is an act in two steps** (§6.3).
- **Letting go:** while gripping, a sprite can push, pull or hit, and choosing anything else lets go. Gripping makes the gripper tired faster.

### 12.2 Breaking free

- **A gripped sprite keeps choosing for itself.** When it tries to move away it struggles, and the world's seeded dice decide whether it breaks free, so replays match. The odds are in the data, shifted by the gripper's arms, the struggler's legs and both sprites' life stages (§6.4). A failed struggle leaves the sprite where it is.

### 12.3 Learning

- **A sprite pushed into thorns learns as from the Cursor's shove:** the thornbush is bad, and it comes to fear the sprite that pushed it. It learns no habit, and a grip that doesn't hurt teaches nothing.
- **Fear of a sprite eases** if that sprite stays near without hurting it ([#72](https://github.com/Keazra/terra-sprites/issues/72)), as fear of the Cursor already does (M1 design §5.6, `cursor_calming`), since M3 adds many lifelong fears: the pusher, the egg breaker, rough play. Pets, shocks and pricks meaning less each time wait; the slice files them as a new issue when it closes #72's half.

### 12.4 Left for its round

- The odds of breaking free and how much each shift counts; how much faster gripping tires.
- The instincts for Grip, Push and Pull, beyond boredom's rung (§11.2).
- How fast fear eases near a harmless sprite.
- How a grip and a struggle show on the map.
- What the made-up thing in B8's test pack is.

---

## 13. Real sleep (Slice 39)

From #237 round 2 Q3 and #288, with #125's, #264's and #277's sleep questions from #232.

### 13.1 Sleep

- **Sleepiness is a feeling of its own.** It builds while a sprite is awake, whatever it's doing, and only sleep eases it. That's the homeostatic half of Borbély's two-process model of sleep, and Creatures kept the two apart too.
- **Sleep comes in a few long stretches,** not many short rests.
- **Sleep costs a fifth less energy than standing awake,** as a missed night's sleep costs a person about a fifth more.
- **Sleep dulls the senses.** A sleeping sprite notices only what's close or loud, such as a hit or its family calling, so a sleeping young sprite needs watching over.
- **Tiredness from exertion stays,** and short breathers ease it, as today. Until Slice 39, today's rests are a fifth cheaper than standing awake (Slice 29).

### 13.2 Left for its round

- How fast sleepiness builds, and how long a sleep lasts. Before M3, sprites rest 15–25% of the time.
- Whether sleep is a verb of its own or a deeper Rest, and what wakes a sprite.
- How sleep shows on the map and in the inspector.
- Whether the sleep genes are needs genes (M2 design §16.6), for careful copying if #237's reserve is ever switched on.
- Whether family members sleep near each other, as a leaning.
- **Learning where to sleep** ([#125](https://github.com/Keazra/terra-sprites/issues/125)): whether sleep can be learned as helping, and aimed at a place or beside family, so sprites learn where and with whom they sleep well.
- **Whether sleep slows drying out too** ([#264](https://github.com/Keazra/terra-sprites/issues/264)); energy is covered. Sitting out a famine itself waits for lean spells, as fat does.
- How sleep counts against #318's bar on grown sprites being on the move (M2 design §10.9), and what it does to §5.4's held-over bars.

---

## 14. Eggs in arms, and eggs that break (Slice 40)

From #227 Q3 and Q10, and #231 Q8, Q14, Q16 and Q17. This changes the glossary's Egg, to which nothing a sprite did did anything.

### 14.1 Carrying

- **Any sprite can pick up an egg,** in one arm (an egg is small), and put it down or hand it over.
- **An egg that's due to hatch while held** hatches on a free tile beside the sprite holding it, which frees the arm, rather than waiting as it does in the Cursor.

### 14.2 What happens to an egg

The owner: "You can throw eggs, like any object that isn't fixed to the ground; they break, and the Sprite inside it dies if it does. Parents and siblings get sad if they see it."

| What happens to the egg | In M3 |
|---|---|
| **Thrown,** by a sprite or by the player with the Cursor | **Breaks 3 times in 4,** where it lands or on what it hits, and the sprite inside dies. The chance is in the egg's data, so a modder can make eggs tougher or frailer. One in four lands whole, so a thrown egg is sometimes a rescue |
| **Put down or dropped,** by a sprite or the player | Never breaks |
| **Pushed** with a grip | Never breaks: a push moves it a tile, it isn't thrown |
| **Knocked by a rolling ball** | Never breaks, as the owner chose on 2026-10-13 |
| **Hit** by a sprite | Nothing, as today. Hard hits wait with weight (#96, #43) |
| **Eaten** | Nothing: an egg isn't food in its data |

- **The death has its own cause,** named in the egg's data, such as "broken as an egg", so the World tab and the family tree say how it died.
- **An egg is never a toy** (§11.1), and bored sprites lean against the hard acts on their own family's eggs (§11.2).

### 14.3 Who learns what

All of it following a break only:
- **Its kin who see it come to fear the thrower,** as a sprite fears one that hurt it, and grieve (§8).
- **A thrower that's kin learns from its own loss:** in the starter genes the loss hurts as pain does.
- **Every thrower feels the empathy pang** (§11.5), kin or not, as strongly as its genes say. The player can add a zap or a shock, which is the player's teaching moment.
- **The young who watch take it on** (M2 design §18): watching passes on what the thrower knows about throwing an egg.

### 14.4 Left for its round

- The words and marks for an egg carried, thrown, broken.
- Whether the Cursor's throw of an egg needs its own warning on screen.
- How a broken egg shows before Slice 42's bodies (nothing is left behind until then).

---

## 15. Pregnancy, with eggs that need warmth (Slice 41)

From #293 and #232's B13, with #325 joining in #233 round 1 Q4.

### 15.1 Pregnancy

- **A mating that works starts a pregnancy, not an egg,** a pair's or one alone. She carries the egg inside her for a while, hungrier and slower, and the Body tab says so. Then she lays it beside her.
- **She can't conceive again until she has laid.**
- **If she dies pregnant, the egg dies with her,** with its own cause of death in the data, and kin who see it grieve both.
- **Pregnancy changes nothing about who is drawn to whom** (§4).

### 15.2 Eggs that need warmth ([#325](https://github.com/Keazra/terra-sprites/issues/325))

- **An egg grows only while a parent is beside it,** as birds' eggs do. Left alone, it hatches late, or goes cold and dies.
- **Why:** nothing threatens the young in M2, so parents evolve to stop looking for them (§5.3). Warm eggs give caring parents a reason to stay. It doesn't touch the lone newborns' bar (10–30%), since the unattended run starts from newborns, not eggs.
- **With pregnancy,** a mother keeps her egg warm inside her, then beside her once it's laid, so the family bars are tuned once for both.
- It makes Slice 41 critical (§3).

### 15.3 Left for its round

- How long a pregnancy lasts (B13's number), and whether the 750 ticks an egg lies are shortened to match.
- What it does to the mother: how much hungrier and slower, and how the player sees it on the map and in the Body and Chem tabs.
- When the egg's cost is paid: all at mating, or spread over the pregnancy.
- Where pregnancy lives: the body (the engine) or the genes.
- How long an egg can be left before it goes cold, whether it can be warmed again, and whether a carried egg counts as warm.
- What happens to an egg the Cursor carries away from its parents.

---

## 16. Dead bodies (Slice 42)

From #85 and #232's B14.

- **Every death leaves a body,** as the species' data says: an object type of its own, in data, that a modder can change or leave out. It lies where the sprite died, and fades after a while, by stages, as other objects do.
- **A selected sprite that dies leaves the selection on its body.** The inspector shows its name, age, generation, family and cause of death.
- **A sprite missing that one which sees the body grieves at once and stops looking** (§8).
- **A body can be gripped, pushed and pulled** (§12), so sprites can drag one, **but it isn't food** in its data, so a sprite that tries to eat one learns it doesn't work.
- **No sense ever takes a body for a living sprite:** family, courtship and G7's tests check it.

**Left for its round:** how long a body lasts (B14's number) and how it looks as it fades; whether a broken egg leaves a shell; whether the Cursor can pick a body up; what, if anything, sprites know of bodies from birth.

---

## 17. Sharing food, with foster carers (Slice 43)

From #227 Q5, Q12 and Q13, with #263 joining from #232. Slice 32 left gifting food for when sprites can carry (the owner, 2026-10-13: "Gifting food could come as a part of socializing moreso than courtship").

- **Handing over is a Drop aimed at a sprite:** the thing goes into its free arm, or at its feet. No new verb.
- **Both sprites feel it,** the giver that it gave and the other that it was given something. The starter genome makes both ease loneliness a little, and genes can change that.
- **The starter species has a small pull to share with its family,** its young and its partner, when one of them is hungry. Its own hunger quiets the pull. So a sprite has to be able to see that another is hungry, as it already sees another's arousal.
- **Foster carers** ([#263](https://github.com/Keazra/terra-sprites/issues/263)): a gene can point at any young sprite, not only its own, so evolution can find carers. The starter species leans to it weakly or not at all, and B6's "3 in 4 hand-overs within the family" keeps it rare. The engine already keeps the breeding drive from drawing the eye to any young sprite, so this is care only (§4).

**Left for its round:** how strong the pull to share is, and how much the giver's hunger quiets it; what the sense of another's hunger is; the motive a foster carer has, if any; the words on screen.

---

## 18. Parents correct their young (Slice 44)

From #67's correcting half and #232's B15.

- **A parent near its young steps in when the young tries something the parent has learned is bad,** like touching a thornbush or throwing an egg. The young stops, and learns from it as from the Cursor's Correct, as a lesson of its own, not one learned by watching (M2 design §19.7 keeps those apart).
- **Correcting is a leaning in the genes, not a rule,** and **a sprite corrects only a try it judges bad itself,** by what it has learned of that thing and that verb. Any sprite fond of another may correct, but the starter species leans to its own young.
- **A correction never injures.** The Cursor's Correct stings without injuring; how much a sprite's correction stings is the round's.
- **Nothing about mating is ever taught by a correction** (§4).
- It's the sprites' own version of the Cursor's Correct, and what the owner hoped for in #229: "they can start teaching each other good manners!"

**Left for its round:** how a correction is done (a Touch with its own effect, or a verb of its own); how much it stings; how strong the leaning is; the words and marks on screen. **Learning a child's likes waits** for favourite things (#65); the slice files it as a new issue when it closes #67's half.

---

## 19. Water play and thornbushes you can eat (Slice 45)

From #64 without juicy berries, and #232's B16.

- **Bored sprites splash in water for fun:** playing with water counts as play, by water's data.
- **A starving sprite may eat a thornbush,** painful as it is: eating one eases some hunger, and pricks as any contact with one does.
- **Both are written only in data,** with no code naming either (B8's way).
- **They cut across the thornbush lesson:** A1's arena has berry bushes, so a sprite that learns well should still prefer them, but it may also learn that thornbushes feed it. A1 and the thorn trap are measured before and after, and their bars stay as they are.

**Left for its round:** how much hunger a thornbush eases and how much it pricks; how strong a splash's play is; whether "a little" of an effect needs a pulse per act (such as `splashed`) or pulses with a strength. **Juicy berries wait** (§1.2); the slice files them as a new issue when it closes #64.

---

## 20. One "take in" verb (Slice 46)

From #91 and #232's B17. Late in M3, so the verb list changes once, in v0.3.0.

- **Eat and Drink retire and Take in joins,** with the old verbs' numbers kept and never reused (M1 design §1.4).
- **Each thing's data says what taking it in is called** ("eat", "drink") **and what it does,** as it says what eating does today. Adding a new food or drink needs no new verb.
- **Every M1 and M2 save opens,** its Eat and Drink read as Take in, as old Play reads as Touch (§6.5).
- **A sprite can no longer try to eat water.** It can still try to take in a ball, which does nothing, so blank-slate mistakes remain, just fewer odd ones.
- **The words:** "Went to drink the water", "Went to eat the berry", worded by the thing.

**Left for its round:** whether other verbs are named by things the same way (a hit on a ball is a kick, a touch on a sprite is play); what an unknown thing's verb is called before the sprite has tried it; how it fits grab before eat's two habits (§10).

---

## 21. An event log that says where (Slice 47)

From #268, whose round settled its shape on 2026-10-17, and #232's B18. Front end only, after Look 3's clicking.

- **Each line says where it happened,** as X and Y, and the sector's name beside them once sectors exist (M4).
- **A click on a line takes the view there** and the tile flashes; a line about a living sprite also selects it.
- **Deaths record a place** (§8.2).
- **The full log opens on `j`,** beside `k` (family tree) and `l` (sprite list). A click on the log's title could open it too.
- **Designed beside the fading alerts** ([#290](https://github.com/Keazra/terra-sprites/issues/290)): if events become alerts over the map that fade away (Look 5's motion), the full log is how to read what has faded.

**Left for its round:** how many events the full log keeps (at least the last hour of play, B18), and whether it's saved with the world; filters in the full view; how places show (a column, or on hover).

---

## 22. A checker for modders' data (Slice 48)

From #276, whose round settled its shape on 2026-10-17, and #232's B19. It comes once most of M3's new data exists.

- **Warnings, never errors,** since odd data may be on purpose: the engine enforces physics only.
- **A flag on the game itself,** `terra-sprites --check-data <folder>`, so modders have it with the game. The checks live in `terra-sim`, which takes the files' text, so tests can use them too.
- **Warnings for data that loads but can never do anything:** an object type nothing places or grows, a stage nothing reaches, a counter nothing changes, a visual state no theme draws, a category with no object types, and a signal a verb sends that no gene reads; and whatever the round adds for M3's new data, such as a thing whose data says a verb is play but no limb can do it.
- **From Slice 48 on,** a slice that adds a new kind of data adds its warnings.

**Left for its round:** the full list, and how a warning names its place in the file; whether the game prints the warnings at start too; whether unmatched genes are warnings.

---

## 23. First steps (Slice 49)

From #317, and #232's B20, with #326's gentle world from #233 round 1 Q5.

- **Three or four small hand-made worlds,** each with one goal on the status line, that teach the Cursor by doing rather than by a wall of help text: such as teaching a sprite that thornbushes hurt, getting two to court, and last, teaching a sprite to push a ball.
- **Started from the title screen** (Look 9); finishing one says so and offers the next.
- **Each world and its goal are data,** as Lab scenarios are, so a new one needs no code.
- **The push world is built to learn fast** (B20): a young sprite, which learns half as fast again as a grown one, one ball beside it, and little else to do.

**Left for its round:** the worlds and their goals; how a goal is checked and what finishing shows; **whether New world offers the gentle world** ([#326](https://github.com/Keazra/terra-sprites/issues/326): the watery world from before Slice 29, reachable today only with `--preset`), and whether a gentle Story world counts as the game as it's meant to be played.

---

## 24. Old worlds

From #233 round 1 Q3, the question #230 and #231 left for this design.

- **A world keeps the rules it was made with.** A save carries its own copy of the data pack, and loading always uses that copy, never the game's newest (M1 design §2.8). Each sprite carries its own genes. So a world made in M2, or in an earlier M3 beta, opens and plays as it was made: its eggs keep `throwable: false`, its berries their size, its sprites their M2 instincts.
- **Each new rule's data has a default that keeps an old world as it was,** as M2's did for M1's worlds (room nearby's numbers, M2 design §10.9).
- **M3's new rules and instincts show in new worlds.** Each beta's notes say to start a new world to see what's new.
- **Not done: moving only the rules.** Sprites from before M3 have no instinct to pick a berry up, so in M3's rules they'd eat fallen berries where they lie and squish about half of them; fallen berries being most of what they eat, such a world would likely go hungry.
- What does carry over is §6.5's: lessons and habits about Play read as Touch, and from Slice 46, Eat and Drink as Take in.
- M4 takes the same line for its ship (M4 design §11, T6).

---

## 25. M3 acceptance, and v0.3.0 (Slice 50)

From #231 Q1 and Q12, and #232.

- **The evidence** in `docs/acceptance/m3.md`, as `docs/acceptance/m2.md` does for M2: B1–B20, M1's and M2's checks (§5.3), §5.4's held-over bars, the speed check, and the long run's information lines (§5.3).
- **The runs:** the viability and unattended runs in the cloud; the lone-world run, the soak and the speed check on the owner's PC.
- **A golden save from M3's end** joins M1's and M2's, so M4's A6 can open it.
- **B11's play-through:** Story mode on the default world, the critical pieces' lines plus a few medium ones, and every first-steps world.
- **v0.3.0,** M3's last beta, once the play-through passes (§2).
- **The issues it closes** are listed on its issue when it's opened.

---

## 26. Later

What M3's design points to, for the milestones after it:

- **Skills** ([#48](https://github.com/Keazra/terra-sprites/issues/48)): practice making an act go better, through §6.4's one place for odds.
- **Limb loss** ([#235](https://github.com/Keazra/terra-sprites/issues/235)) and **organs** ([#31](https://github.com/Keazra/terra-sprites/issues/31)), on §6.2's per-limb tracking.
- **Ball games** ([#45](https://github.com/Keazra/terra-sprites/issues/45)) and **choosing where to stand** ([#246](https://github.com/Keazra/terra-sprites/issues/246)), on §11.6's social play with a ball.
- **Favourite things** ([#65](https://github.com/Keazra/terra-sprites/issues/65)), with a parent learning its child's likes.
- **The rest of grief** ([#206](https://github.com/Keazra/terra-sprites/issues/206)): who matters, a lasting memory whose pain fades, mourning places.
- **The ship** (M4): panels, switches and garden beds worked with M3's verbs ([M4 design](m4-terra.md) §6.3, §7.2).
