# The design toolkit

The practical companion to [`DESIGN.md`](../DESIGN.md). DESIGN.md says what the
system _is_; this says what to reach for when you are building.

Everything here is importable. Nothing here needs a build step, a dependency,
or a decision from you.

---

## 1. The tokens

```ts
import {
  colors,
  radii,
  spacing,
  shadows,
  praxisGradients,
  typography,
  alpha,
} from "../../components/design-system/tokens";
```

| Import            | Use it for                                                                                                                                                                                                      |
| ----------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `colors`          | every fill, seam and text colour. `colors.bg` `surface` `raised` `border` `muted` `fg` `sub` `dim` `amber` `violet` `red` `green` `blue` `teal` `pink` `orange`, plus the light/dark steps of amber and violet. |
| `alpha(hex, a)`   | a translucent tint. `alpha(colors.amber, 0.12)`. Never hand-write the rgba conversion — there used to be eight spellings of it.                                                                                 |
| `radii`           | `sm` 8 · `md` 12 · `lg` 16 · `xl` 20 · `full` 9999. Numbers, not strings: `` `${radii.md}px` ``.                                                                                                                |
| `spacing`         | `xs` 4 · `sm` 8 · `md` 16 · `lg` 24 · `xl` 32 · `2xl` 48.                                                                                                                                                       |
| `shadows`         | the shadow vocabulary, including `shadows.glow.*`.                                                                                                                                                              |
| `praxisGradients` | the amber and violet button gradients.                                                                                                                                                                          |
| `typography`      | `fontFamily` (Inter), `headingFont` (Plus Jakarta Sans), `monoFont` (JetBrains Mono).                                                                                                                           |

**The Sync Rule.** The ramp lives in three places and they must stay identical:
`client/src/components/design-system/tokens.ts`, the `@theme` block in
`client/src/index.css`, and the `createTheme` palette in `client/src/index.tsx`.
If you add a colour, add it to all three. A colour that exists in only one of
them is a seam you will see on screen.

> `tokens.colors.green` was `#22c55e` while the MUI palette said `#10b981` for
> months. Nothing caught it because most pages hardcoded their own values.
> **The tokens only work if you stop hardcoding.**

---

## 2. The shared components

`client/src/components/design-system/` — import from the barrel:

```ts
import {
  PraxisCard,
  PraxisButton,
  PraxisChip,
  PraxisBadge,
  PraxisSection,
  PraxisHeader,
  PraxisDivider,
  PraxisDialog,
} from "../../components/design-system";
```

| Component       | Guarantees                                                                                                                                   |
| --------------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| `PraxisCard`    | surface fill, 12px radius, 1px seam. Flat at rest; lifts 2px on hover.                                                                       |
| `PraxisButton`  | `primary` (amber gradient), `secondary` (violet gradient), `outline`, `ghost`. 12px radius, ramp text colours. `loading` swaps in a spinner. |
| `PraxisChip`    | a full pill with a 12%-tint fill and a coloured label. `colorScheme` picks the hue.                                                          |
| `PraxisBadge`   | the uppercase micro-label, mono, for a tier or a status word.                                                                                |
| `PraxisSection` | title + subtitle + action + a divider, so every section keeps the same rhythm.                                                               |
| `PraxisHeader`  | the page title block. The `gradient` prop marks a tier word with a tinted pill — never gradient text.                                        |
| `PraxisDialog`  | the modal shell. Paper is `colors.raised`.                                                                                                   |
| `PraxisDivider` | a 1px seam rule.                                                                                                                             |

`PraxisButton`'s `secondary` is the **violet** accent; DESIGN.md's
`button-secondary` is the **quiet raised** sibling. They are different things
under the same word — use `outline` or `ghost` for the quiet one.

---

## 3. The motion kit

`client/src/lib/motion.tsx`:

```ts
import {
  Rise,
  Cascade,
  Item,
  Reveal,
  page,
  EASE_OUT,
  spring,
} from "../../lib/motion";
```

| Primitive          | Use it for                                                                                              |
| ------------------ | ------------------------------------------------------------------------------------------------------- |
| `Rise`             | a block entrance on mount. `delay` sequences sections.                                                  |
| `Cascade` / `Item` | a list or grid entrance — siblings stagger 50ms apart.                                                  |
| `Reveal`           | a scroll-triggered entrance, for long pages. Fires once, when the block first crosses into view.        |
| `CountUp`          | a measured figure that counts to its value. Renders the settled value immediately under reduced motion. |
| `page`             | the page enter/exit pair for `AnimatePresence` in `AppShell`.                                           |
| `EASE_OUT`         | `cubic-bezier(0.22, 1, 0.36, 1)` — the one curve in the app.                                            |

**One authored moment per surface.** Not an identical entrance on every
section. Pick the moment that the page is actually about:

- a long landing → the hero cascades, and the headings reveal as you reach them;
- a task page → one rise on the first block, and nothing after it;
- a page with a number that carries the claim → the number counts.

**The rules.** Transform and opacity only — never `width`, `height`, `padding`
or `margin` (a collapse uses `grid-template-rows`). No bounce or elastic
easing. Reduced motion is handled globally: `<MotionConfig reducedMotion="user">`
covers framer-motion, and the CSS kill-switch in `client/src/index.tsx` covers
every keyframe. **Do not add your own `prefers-reduced-motion` block for a
framer-motion component** — it is already dead.

---

## 4. Brand marks

`client/src/components/brand/`:

```ts
import { WhiteHorse } from '../../components/brand';
<WhiteHorse size={54} color={colors.fg} label="Verona K" />
```

A mark is **drawn, not typed**. The white horse is a filled silhouette with the
eye and the nostril punched out of the fill (`fill-rule="evenodd"`), so it reads
on any surface and needs no colour of its own. It holds at 24px, which is the
size that matters.

**Emoji are not marks.** They render in the platform's own colour and weight
and read as a sticker where a signature is meant. Emoji _label content_ — a
domain, a tool, a sentence, a fact. The moment they are a row of navigation
icons, they are an icon system and they have to be drawn: `@mui/icons-material`
for the app chrome, an authored SVG for a brand.

---

## 5. The sub-brands

Not every surface wears the Midnight Dojo ramp, and that is deliberate.

| World             | Where                                          | Its rules                                                                                                            |
| ----------------- | ---------------------------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| **Midnight Dojo** | the app, `/me`, `/veronak`, the landing funnel | near-black, one amber lamp, one violet banner. This document.                                                        |
| **Paper**         | `/physis`                                      | a light engineering document: paper ground, forest panels, lime and blue accents. Self-hosts its own variable fonts. |
| **Earth**         | `/agliardi`                                    | a real earthworks company: earth brown, warm paper, dark slate. Its own world, its own copy, in Italian.             |
| **Mint**          | `/bioenergetics`                               | a body-mind manual: mint family over deep navy. The body has its own voice and keeps it.                             |

Each must be internally consistent. None of them is required to be Midnight
Dojo. But each must **say plainly which product it is talking about** — the
name "Physis" covers two different things, and a visitor who finds one must
not be able to mistake it for the other.

---

## 6. Before you open a file

- [ ] Am I reaching for a shared component instead of re-deriving a card fill?
- [ ] Have I imported the colour, or hardcoded it?
- [ ] Is this page's motion one authored moment?
- [ ] If I added a tint, did I use `alpha()`?
- [ ] If I added a token, did I add it to all three sources?
- [ ] Is the icon drawn, or is it an emoji standing in for one?
- [ ] Have I looked at the rendered result at the size it ships at?

The last one is not optional. A build that compiles is not a page that renders,
and a detector that passes is not a page that reads.
