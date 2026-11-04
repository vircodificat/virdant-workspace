# Alternatives for `when` / `match` arm syntax

This document collects every syntax alternative discussed for Virdant's
conditional (`when`) and pattern-matching (`match`) constructs.

## The problem

Today both constructs use `case` to introduce an arm, but the two `case`s mean
different things:

* In `when`, `case <expr>` tests a `Bit`-typed **condition**.
* In `match`, `case <pat>` matches a **pattern** against the scrutinee and may
  bind variables.

Sharing one keyword for two different concepts is the thing we want to fix.
Secondary goals, in rough priority order:

1. One keyword, one meaning.
2. Arms should read as natural English.
3. Keep the flat, vertically-aligned arm list that makes FSM code read like a table.
4. Keep the "direct chaining" idiom (`case d_done match serving { … }`).
5. Keep the priority chain a single lexical unit (a header + braces), so two
   adjacent conditions can never be misread as independent when they are a chain.
6. Prefer familiar HDL vocabulary where it doesn't conflict with the above.
7. Avoid keywords that are common signal names.

Every entry below gives a **statement-level** example and an **expression-level**
example, followed by rationale.

Running examples used throughout:

```virdant
// statement-level: a small FSM timer
when {
    case reset {
        phase <= #Red
        timer <= 8
    }
    case timer == 0 {
        phase <= next_phase
    }
    else {
        timer <= timer - 1
    }
}

// expression-level: a max
max := when {
    case a > b => a
    else       => b
}

// statement-level match
match state {
    case @Done(r) {
        result := r
        valid  := true
    }
    case @Idle() {
        valid := false
    }
}

// expression-level match
valid := match s.data {
    case @Valid(dontcare) => true
    case @Invalid()       => false
}
```

---

## Part 0 — Baseline

### 0.1 Status quo: `case` in both

```virdant
when {
    case reset      { phase <= #Red  timer <= 8 }
    case timer == 0 { phase <= next_phase }
    else            { timer <= timer - 1 }
}

max := when {
    case a > b => a
    else       => b
}

match state {
    case @Done(r) { result := r  valid := true }
    case @Idle()  { valid := false }
}

valid := match s.data {
    case @Valid(dontcare) => true
    case @Invalid()       => false
}
```

**Rationale.** One shape for both constructs (header, `case` arms, optional
`else`). `case` is what Verilog/VHDL/SV call their multi-way branch. `case` and
`else` are both four letters, so arms align. Direct chaining composes cleanly.

**Against.** `case` means two different things depending on the header. The
reader must look up to know whether `case x` is a condition or a binding
pattern. This is the complaint that motivates the rest of the document.

---

## Part 1 — Changing the `match` arm keyword

### 1.1 `with` as the `match` arm keyword (original proposal)

```virdant
when {
    case reset      { phase <= #Red  timer <= 8 }
    case timer == 0 { phase <= next_phase }
    else            { timer <= timer - 1 }
}

max := when {
    case a > b => a
    else       => b
}

match state {
    with @Done(r) { result := r  valid := true }
    with @Idle()  { valid := false }
}

valid := match s.data {
    with @Valid(dontcare) => true
    with @Invalid()       => false
}
```

**Rationale.** Distinguishes pattern arms from condition arms. `with` and `else`
are both four letters, so alignment survives. Vaguely echoes OCaml's `match … with`.

**Against.** In every language where it appears (`Python`, `JS`, `Pascal`,
OCaml) `with` introduces a *scope or context*, not an *alternative*; OCaml uses
it once after the scrutinee, never per arm. "With Valid(signal), false" is not
how anyone describes a branch. It also spends novelty budget on the one keyword
where Verilog's choice (`case`) was already fine. The `when` side still says
`case <condition>`, which is the weaker of the two phrases, so the construct
that most needed help gets none.

### 1.2 `is` as the `match` arm keyword

```virdant
when {
    case reset      { phase <= #Red  timer <= 8 }
    case timer == 0 { phase <= next_phase }
    else            { timer <= timer - 1 }
}

max := when {
    case a > b => a
    else       => b
}

match state {
    is @Done(r) { result := r  valid := true }
    is @Idle()  { valid := false }
}

valid := match s.data {
    is @Valid(dontcare) => true
    is @Invalid()       => false
}
```

**Rationale.** "state is Done(r)" reads as natural English. Pattern arms become
unmistakable. Leaves `when`'s `case` untouched.

**Against.** Gives up `case` precisely where HDL readers expect it most (value
matching). `case <condition>` on the `when` side remains the awkward phrase.
`is` is two letters, breaking alignment with `else`.

### 1.3 Bare arms, Rust-style (no arm keyword in `match`)

```virdant
when {
    case reset      { phase <= #Red  timer <= 8 }
    case timer == 0 { phase <= next_phase }
    else            { timer <= timer - 1 }
}

max := when {
    case a > b => a
    else       => b
}

match state {
    @Done(r) { result := r  valid := true }
    @Idle()  { valid := false }
}

valid := match s.data {
    @Valid(dontcare) => true
    @Invalid()       => false
}
```

**Rationale.** Removes the redundant word entirely; patterns already carry
sigils (`@`, `#`) so they are visually distinct. Rust readers will feel at home.

**Against.** Breaks the `when`/`match` symmetry. Block-bodied arms lose their
leading anchor, so `@Done(r) {` is harder to scan than `case @Done(r) {`.
Literal patterns (`0 {`, `x {`) look like stray expressions. The `else` arm is
now the only one with a keyword, which is lopsided.

---

## Part 2 — Changing the `when` arm keyword (header stays `when`)

### 2.1 `when { if … }`

```virdant
when {
    if reset      { phase <= #Red  timer <= 8 }
    if timer == 0 { phase <= next_phase }
    else          { timer <= timer - 1 }
}

max := when {
    if a > b => a
    else     => b
}

match state {
    case @Done(r) { result := r  valid := true }
    case @Idle()  { valid := false }
}

valid := match s.data {
    case @Valid(dontcare) => true
    case @Invalid()       => false
}
```

**Rationale.** `if` is already a reserved token in the lexer and unused in the
grammar, so no identifiers break. `case` is now unique to pattern arms and keeps
its HDL meaning; `if` is unique to condition arms. The flat block and direct
chaining (`if d_done match serving { … }`) survive.

**Against.** "when if" is not natural English. A reader skimming
`when { if a {…} if b {…} }` may read the arms as *independent* `if`s rather
than a first-match chain; in hardware the both-fire vs. one-fires distinction
matters. `if` is two letters, so arms no longer align with `else`.

### 2.2 `when { on … }`

```virdant
when {
    on reset      { phase <= #Red  timer <= 8 }
    on timer == 0 { phase <= next_phase }
    else          { timer <= timer - 1 }
}

max := when {
    on a > b => a
    else     => b
}
```

**Rationale.** "when, on reset…" reads naturally. Short. Event-ish flavour
suits FSM code.

**Against.** `on` is already a keyword: `reg r : T on clock` means "clocked
by". `when { on reset … }` therefore risks reading as "on the reset *edge*",
which is exactly the level-vs-edge confusion an HDL should not invite. Two
letters, breaks alignment.

### 2.3 `when { at … }`

```virdant
when {
    at reset      { phase <= #Red  timer <= 8 }
    at timer == 0 { phase <= next_phase }
    else          { timer <= timer - 1 }
}

max := when {
    at a > b => a
    else     => b
}
```

**Rationale.** "when, at count == 0…" reads well for the equality and
threshold tests that dominate FSM code. Not yet a keyword; no collision with
`on clock`. Two letters.

**Against.** A preposition, so it fits some conditions badly ("at a > b").
Mild temporal connotation ("at time t"). Breaks alignment.

### 2.4 `when { in … }`

```virdant
when {
    in reset      { phase <= #Red  timer <= 8 }
    in timer == 0 { phase <= next_phase }
    else          { timer <= timer - 1 }
}

max := when {
    in a > b => a
    else     => b
}
```

**Rationale.** "when in reset", "when in halted" is lovely for state-like
boolean signals.

**Against.** "when in a > b" is nonsense. Only works if most conditions are
named signals, which is not something the language can guarantee.

### 2.5 `when { under … }`

```virdant
when {
    under reset      { phase <= #Red  timer <= 8 }
    under timer == 0 { phase <= next_phase }
    else             { timer <= timer - 1 }
}

max := when {
    under a > b => a
    else        => b
}
```

**Rationale.** "when under reset" is natural English.

**Against.** "under a > b" is not. Five letters, long for an arm keyword.

### 2.6 `when { with … }`

```virdant
when {
    with reset      { phase <= #Red  timer <= 8 }
    with timer == 0 { phase <= next_phase }
    else            { timer <= timer - 1 }
}

max := when {
    with a > b => a
    else       => b
}
```

**Rationale.** The original `with` proposal relocated to the side where it
does less damage. Four letters, aligns with `else`. No collision.

**Against.** "when, with reset…" is flat rather than wrong. Still carries the
scope/context connotation from other languages.

### 2.7 `when { given … }`

```virdant
when {
    given reset      { phase <= #Red  timer <= 8 }
    given timer == 0 { phase <= next_phase }
    else             { timer <= timer - 1 }
}

max := when {
    given a > b => a
    else        => b
}
```

**Rationale.** The most universally natural English of the arm-keyword
options; works for every kind of condition ("given reset", "given a > b").

**Against.** Five letters; misaligns with `else`. Slight "precondition"
flavour.

### 2.8 `when { where … }`

```virdant
when {
    where reset      { phase <= #Red  timer <= 8 }
    where timer == 0 { phase <= next_phase }
    else             { timer <= timer - 1 }
}

max := when {
    where a > b => a
    else        => b
}
```

**Rationale.** SQL/Haskell familiarity.

**Against.** "when, where a > b" does not parse as English. Five letters.

### 2.9 `when { while … }`

```virdant
when {
    while reset      { phase <= #Red  timer <= 8 }
    while timer == 0 { phase <= next_phase }
    else             { timer <= timer - 1 }
}

max := when {
    while a > b => a
    else        => b
}
```

**Rationale.** For level-sensitive combinational logic, "while busy" is an
accurate description of what the hardware does.

**Against.** Every software reader sees a loop. Five letters.

### 2.10 `when { during … }`

```virdant
when {
    during reset      { phase <= #Red  timer <= 8 }
    during timer == 0 { phase <= next_phase }
    else              { timer <= timer - 1 }
}

max := when {
    during a > b => a
    else         => b
}
```

**Rationale.** Same level-sensitive accuracy as `while`, without the loop
baggage.

**Against.** Six letters. "during a > b" is strained.

### 2.11 `when { once … }`

```virdant
when {
    once reset      { phase <= #Red  timer <= 8 }
    once timer == 0 { phase <= next_phase }
    else            { timer <= timer - 1 }
}

max := when {
    once a > b => a
    else       => b
}
```

**Rationale.** Four letters, aligns.

**Against.** Implies a one-shot or latch. Actively misleading for
combinational logic. Avoid.

### 2.12 `when { iff … }`

```virdant
when {
    iff reset      { phase <= #Red  timer <= 8 }
    iff timer == 0 { phase <= next_phase }
    else           { timer <= timer - 1 }
}

max := when {
    iff a > b => a
    else      => b
}
```

**Rationale.** Short, logic-flavoured.

**Against.** Mathematically wrong: an arm is not a biconditional. Avoid.

### 2.13 Postfix `holds`

```virdant
when {
    reset holds      { phase <= #Red  timer <= 8 }
    timer == 0 holds { phase <= next_phase }
    else             { timer <= timer - 1 }
}

max := when {
    a > b holds => a
    else        => b
}
```

**Rationale.** Very readable English.

**Against.** Postfix means arms have no leading anchor; the eye cannot find
arm starts by scanning the left column. Arms of different lengths put `holds`
in different columns.

### 2.14 `unless` as an additional negated arm

```virdant
when {
    on go        { phase <= #Run }
    unless ready { phase <= #Wait }
    else         { phase <= #Idle }
}

ok := when {
    unless valid => false
    else         => true
}
```

**Rationale.** Not a replacement but an addition: lets you write
`unless valid` instead of `on !valid`. Pairs with any positive arm keyword.

**Against.** Niche. Two keywords for arms increases surface area. Mixed
positive and negative arms in one block are harder to read than they look.

### 2.15 Ordinals: `first` / `then` / `else`

```virdant
when {
    first reset      { phase <= #Red  timer <= 8 }
    then  timer == 0 { phase <= next_phase }
    then  timer == 1 { phase <= #Yellow }
    else             { timer <= timer - 1 }
}

max := when {
    first a > b => a
    else        => b
}
```

**Rationale.** The only option that makes "first match wins" explicit on the
page. Reads as a genuine English sentence.

**Against.** The first arm has a different keyword from the rest, so
reordering arms means editing keywords. Unusual enough that every new reader
stops. A two-arm block has no `then`, which looks odd. Keep in the drawer as a
curiosity.

### 2.16 `?` prefix marker

```virdant
when {
    ? reset      { phase <= #Red  timer <= 8 }
    ? timer == 0 { phase <= next_phase }
    else         { timer <= timer - 1 }
}

max := when {
    ? a > b => a
    else    => b
}
```

**Rationale.** "when — reset? — do this." Literally "is this true?". Short,
has a leading anchor, no English-phrase problem at all. Aligns with padding.

**Against.** Novel. Spends the `?` symbol, which you may want for a future
ternary or optional-type syntax.

### 2.17 `?` postfix marker

```virdant
when {
    reset?      { phase <= #Red  timer <= 8 }
    timer == 0? { phase <= next_phase }
    else        { timer <= timer - 1 }
}

max := when {
    a > b? => a
    else   => b
}
```

**Rationale.** Even more natural English, Ruby-style predicate feel.

**Against.** Same no-leading-anchor problem as postfix `holds`.
`timer == 0?` is visually ambiguous with a hypothetical `0?` postfix operator.

### 2.18 `|` prefix bar (guards)

```virdant
when {
    | reset      { phase <= #Red  timer <= 8 }
    | timer == 0 { phase <= next_phase }
    | else       { timer <= timer - 1 }
}

max := when {
    | a > b => a
    | else  => b
}

match state {
    | @Done(r) { result := r  valid := true }
    | @Idle()  { valid := false }
}

valid := match s.data {
    | @Valid(dontcare) => true
    | @Invalid()       => false
}
```

**Rationale.** Haskell/OCaml convention. `|` just means "arm"; what follows is
whatever the header says, so the keyword question disappears entirely.
Perfect alignment for free. Works identically for `when` and `match`.

**Against.** Fresh to Verilog readers. `|` is already bitwise-or, which is
lexically fine (statement start) but visually busy. Direct chaining
(`| d_done match serving { | … }`) looks odd. A bigger visual change than any
keyword swap.

### 2.19 No arm keyword at all

```virdant
when {
    reset      { phase <= #Red  timer <= 8 }
    timer == 0 { phase <= next_phase }
    else       { timer <= timer - 1 }
}

max := when {
    a > b => a
    else  => b
}
```

**Rationale.** Virdant's struct literal is `$ { … }`, so `expr {` has no parse
ambiguity. The arrow form reads fine.

**Against.** The block form loses its leading anchor and `timer == 0 { … }`
looks like a struct literal to readers of other languages. `else` becomes the
only arm with a keyword, which is lopsided.

---

## Part 3 — Replacing `when` with a different construct

### 3.1 `if / else if / else` (drop `when`)

```virdant
if reset {
    phase <= #Red
    timer <= 8
} else if timer == 0 {
    phase <= next_phase
} else {
    timer <= timer - 1
}

max := if a > b { a } else { b }

match state {
    case @Done(r) { result := r  valid := true }
    case @Idle()  { valid := false }
}

valid := match s.data {
    case @Valid(dontcare) => true
    case @Invalid()       => false
}
```

**Rationale.** Fully conventional; zero ambiguity about priority; `case`
becomes unique to `match`. Direct chaining still works
(`else if d_done match serving { … }`).

**Against.** Loses the flat, aligned arm table that makes `spi.vir` and
`mux.vir` read well. Loses the priority chain as a single lexical unit with a
header. The expression form is chattier and nests badly beyond two arms.
Gives up one of Virdant's genuine readability wins.

### 3.2 Scrutinee-less `match { case … }`

```virdant
match {
    case reset      { phase <= #Red  timer <= 8 }
    case timer == 0 { phase <= next_phase }
    else            { timer <= timer - 1 }
}

max := match {
    case a > b => a
    else       => b
}
```

**Rationale.** One keyword fewer. SystemVerilog's `case (1'b1)`
priority-encoder idiom and Lisp's `cond` are exactly this.

**Against.** Does not fix the complaint: `case` still means two things, and the
disambiguation has merely moved from `when`/`match` to "is there a scrutinee?",
which is subtler at a glance. Rejected for the stated goal — but see 4.17 for
the version that *does* work.

### 3.3 Guards on `match` arms (`case pat if cond`)

```virdant
match cmd {
    case @Get(addr) if addr == 0 { hit := true }
    case @Get(addr)              { hit := lookup(addr) }
    case @Put(addr, data)        { hit := false }
}

value := match cmd {
    case @Get(addr) if addr == 0 => 0
    case @Get(addr)              => mem[addr]
    case @Put(addr, data)        => data
}
```

**Rationale.** Gives `case` a single unified *semantics*: "pattern, optionally
guarded by a condition". `when { case c … }` can then be defined as sugar for
`case _ if c`. Guards are independently useful; today you must nest a `when`
inside a match arm to get this.

**Against.** Unifies the spec, not the page: unless the `when` surface syntax
also changes, the reader still sees `case <bool>` in one place and
`case <pat>` in another. Best paired with 2.1 or any of Part 4.

### 3.4 Bare adjacent `when` chains (no header)

```virdant
when reset      { phase <= #Red  timer <= 8 }
when timer == 0 { phase <= next_phase }
else            { timer <= timer - 1 }

max := when a > b => a
       else       => b
```

**Rationale.** Fewest braces; arms read as English.

**Against.** Adjacency now defines chaining. Two consecutive `when` statements
are ambiguous between "one priority chain" and "two independent conditions",
and in hardware that is exactly the distinction that must never blur. The
header is what makes the chain a single lexical unit. **Avoid.**

### 3.5 VHDL-style postfix conditional (`a when c else b`)

```virdant
// statement-level has no good form; VHDL uses it for assignments only
phase <= #Red when reset else next_phase when timer == 0 else phase

max := a when a > b else b
```

**Rationale.** Idiomatic VHDL. For the binary case it is lovely and compact.

**Against.** For N ≥ 3 arms the chain becomes unscannable. Only worth
considering as *additional* sugar for the two-way case, never as a
replacement for the block form.

---

## Part 4 — `when` becomes the arm keyword; a new header word

The insight: `when` is the word that wants to be on the *arm*. "When reset, do
this; when the timer hits zero, do that; else…" is plain English and is also how
VHDL spells conditional alternatives (`y <= a when c1 else b when c2 else c;`).
Every arm keyword then reads as a phrase — *when* `<condition>`, *case*
`<pattern>`, *else* — and `when`/`case`/`else` are all four letters, so
alignment survives. Direct chaining improves:
`when d_done match serving { case … }` is literally "when d_done, match serving".

The only remaining question is the header word.

### 4.1 `choose { when … }`

```virdant
choose {
    when reset      { phase <= #Red  timer <= 8 }
    when timer == 0 { phase <= next_phase }
    else            { timer <= timer - 1 }
}

max := choose {
    when a > b => a
    else       => b
}
```

**Rationale.** Reads as English in both positions ("choose: when…"). Unlikely
to be a signal name. Neutral about hardware structure, so it never misleads.

**Against.** No precedent in another language. A verb, so the expression form
is a touch weaker than a noun would be. Six letters.

### 4.2 `cond { when … }`

```virdant
cond {
    when reset      { phase <= #Red  timer <= 8 }
    when timer == 0 { phase <= next_phase }
    else            { timer <= timer - 1 }
}

max := cond {
    when a > b => a
    else       => b
}
```

**Rationale.** The canonical name for this construct in Lisp/Scheme/Clojure.
Terse; four letters.

**Against.** Opaque to hardware engineers who have not seen Lisp. `cond` is a
plausible signal name. Not an English word.

### 4.3 `select { when … }`

```virdant
select {
    when reset      { phase <= #Red  timer <= 8 }
    when timer == 0 { phase <= next_phase }
    else            { timer <= timer - 1 }
}

max := select {
    when a > b => a
    else       => b
}
```

**Rationale.** Familiar to HDL readers.

**Against.** VHDL's `with … select` picks on a *value*, i.e. it is `match`, so
the word points the wrong way. `select`/`sel` is one of the most common signal
names in existence.

### 4.4 `first { when … }`

```virdant
first {
    when reset      { phase <= #Red  timer <= 8 }
    when timer == 0 { phase <= next_phase }
    else            { timer <= timer - 1 }
}

max := first {
    when a > b => a
    else       => b
}
```

**Rationale.** Makes the first-match-wins semantics explicit.

**Against.** `max := first { … }` reads oddly. `first` is a common identifier
(`first_cycle`, `first`). Five letters.

### 4.5 `priority { when … }`

```virdant
priority {
    when reset      { phase <= #Red  timer <= 8 }
    when timer == 0 { phase <= next_phase }
    else            { timer <= timer - 1 }
}

max := priority {
    when a > b => a
    else       => b
}
```

**Rationale.** SystemVerilog precedent (`priority if`, `priority case`).
Explicit about semantics.

**Against.** Long. SV's `priority` also implies full-coverage assertions you
probably don't want to promise. Awkward in expression position.

### 4.6 `decide { when … }`

```virdant
decide {
    when reset      { phase <= #Red  timer <= 8 }
    when timer == 0 { phase <= next_phase }
    else            { timer <= timer - 1 }
}

max := decide {
    when a > b => a
    else       => b
}
```

**Rationale.** A verb that reads as English.

**Against.** Slightly ponderous. Six letters. No precedent.

### 4.7 `pick { when … }`

```virdant
pick {
    when reset      { phase <= #Red  timer <= 8 }
    when timer == 0 { phase <= next_phase }
    else            { timer <= timer - 1 }
}

max := pick {
    when a > b => a
    else       => b
}
```

**Rationale.** Short (four letters), informal, reads fine in both positions.

**Against.** Plausible signal name (`pick`, `pick_valid`). Informal register
may feel out of place next to `incoming`/`outgoing`.

### 4.8 `branch { when … }`

```virdant
branch {
    when reset      { phase <= #Red  timer <= 8 }
    when timer == 0 { phase <= next_phase }
    else            { timer <= timer - 1 }
}

max := branch {
    when a > b => a
    else       => b
}
```

**Rationale.** Clear CPU-flavoured meaning.

**Against.** Collides hard with branch predictors, `branch_taken`,
`branch_target` in real designs. Avoid.

### 4.9 `route { when … }`

```virdant
route {
    when reset      { phase <= #Red  timer <= 8 }
    when timer == 0 { phase <= next_phase }
    else            { timer <= timer - 1 }
}

max := route {
    when a > b => a
    else       => b
}
```

**Rationale.** Nice for datapath code ("route the data: when…").

**Against.** Wrong metaphor for state machines and for scalar expressions.

### 4.10 `prefer { when … }`

```virdant
prefer {
    when reset      { phase <= #Red  timer <= 8 }
    when timer == 0 { phase <= next_phase }
    else            { timer <= timer - 1 }
}

max := prefer {
    when a > b => a
    else       => b
}
```

**Rationale.** Explicitly says earlier arms win.

**Against.** `max := prefer { … }` reads oddly. Six letters.

### 4.11 `order { when … }`

```virdant
order {
    when reset      { phase <= #Red  timer <= 8 }
    when timer == 0 { phase <= next_phase }
    else            { timer <= timer - 1 }
}

max := order {
    when a > b => a
    else       => b
}
```

**Rationale.** Hints at ordering.

**Against.** Too generic. Collides with memory-ordering vocabulary. Avoid.

### 4.12 `chain { when … }`

```virdant
chain {
    when reset      { phase <= #Red  timer <= 8 }
    when timer == 0 { phase <= next_phase }
    else            { timer <= timer - 1 }
}

max := chain {
    when a > b => a
    else       => b
}
```

**Rationale.** Describes the `else-if` chain structure literally.

**Against.** Neutral but un-English-y; "chain: when…" is not a phrase anyone
says. Collides with scan chains, daisy chains.

### 4.13 `ladder { when … }`

```virdant
ladder {
    when reset      { phase <= #Red  timer <= 8 }
    when timer == 0 { phase <= next_phase }
    else            { timer <= timer - 1 }
}

max := ladder {
    when a > b => a
    else       => b
}
```

**Rationale.** PLC "ladder logic" connotation; evokes a vertical list of rungs
evaluated in order.

**Against.** Obscure outside industrial control. Cute rather than clear.

### 4.14 `guard` / `guards { when … }`

```virdant
guards {
    when reset      { phase <= #Red  timer <= 8 }
    when timer == 0 { phase <= next_phase }
    else            { timer <= timer - 1 }
}

max := guards {
    when a > b => a
    else       => b
}
```

**Rationale.** Haskell's name for exactly this (`| a > b = a`). "guards:
when…, else…" reads acceptably.

**Against.** `guard` singular is a common identifier. Plural keyword is
unusual. PL-flavoured rather than HDL-flavoured.

### 4.15 `conds { when … }`

```virdant
conds {
    when reset      { phase <= #Red  timer <= 8 }
    when timer == 0 { phase <= next_phase }
    else            { timer <= timer - 1 }
}

max := conds {
    when a > b => a
    else       => b
}
```

**Rationale.** Plural of `cond`; signals "a list of them".

**Against.** Ugly. Avoid.

### 4.16 `test` / `tests { when … }`

```virdant
test {
    when reset      { phase <= #Red  timer <= 8 }
    when timer == 0 { phase <= next_phase }
    else            { timer <= timer - 1 }
}

max := test {
    when a > b => a
    else       => b
}
```

**Rationale.** Says "test these conditions".

**Against.** Overloaded with the test-suite meaning; a `test` keyword in a
language with a test directory invites confusion. Avoid.

### 4.17 `match { when … }` (scrutinee-less `match`, arms differ)

```virdant
match {
    when reset      { phase <= #Red  timer <= 8 }
    when timer == 0 { phase <= next_phase }
    else            { timer <= timer - 1 }
}

max := match {
    when a > b => a
    else       => b
}

match state {
    case @Done(r) { result := r  valid := true }
    case @Idle()  { valid := false }
}
```

**Rationale.** Unlike 3.2, this *does* resolve the complaint: the arm keywords
alone (`when` vs `case`) now disambiguate, and the header is shared. One fewer
keyword in the language. Mirrors Go's tagless `switch`.

**Against.** `match` with no subject is slightly surprising on first sight.
The reference must spell out that a subject-less `match` is a priority chain.
Readers may wonder what is being matched.

### 4.18 `if { when … }`

```virdant
if {
    when reset      { phase <= #Red  timer <= 8 }
    when timer == 0 { phase <= next_phase }
    else            { timer <= timer - 1 }
}

max := if {
    when a > b => a
    else       => b
}
```

**Rationale.** `if` is already reserved. Every reader instantly knows it is a
conditional. Two letters.

**Against.** "if: when reset…" is mildly redundant English. `if {` with no
condition directly after it will surprise C-trained readers.

### 4.19 `mux { when … }`

```virdant
mux {
    when reset      { phase <= #Red  timer <= 8 }
    when timer == 0 { phase <= next_phase }
    else            { timer <= timer - 1 }
}

max := mux {
    when a > b => a
    else       => b
}
```

**Rationale.** Virdant already has `mux(c, a, b)`; a block form unifies the
two. Hardware engineers know exactly what a mux is.

**Against.** `mux` connotes a *parallel* select-by-index, not a priority
chain; slightly misleading for `when` arms. Statement-level `mux { … }`
driving multiple registers is a stretch of the word.

### 4.20 `switch { when … }`

```virdant
switch {
    when reset      { phase <= #Red  timer <= 8 }
    when timer == 0 { phase <= next_phase }
    else            { timer <= timer - 1 }
}

max := switch {
    when a > b => a
    else       => b
}
```

**Rationale.** **Go's tagless `switch`** is exactly this construct: no subject,
boolean cases, first match wins. HDL readers know `switch` loosely from C. The
missing subject tells C readers "this is not value-switch".

**Against.** C-trained readers initially expect a value. Six letters. Having
both `switch` and `match` in one language invites "what's the difference?"
(answer: `switch` has no subject — which is a clean rule, but still a question).

### 4.21 `rules { when … }`

```virdant
rules {
    when reset      { phase <= #Red  timer <= 8 }
    when timer == 0 { phase <= next_phase }
    else            { timer <= timer - 1 }
}

max := rules {
    when a > b => a
    else       => b
}
```

**Rationale.** Bluespec's `rule`. Reads as a rule table, which is how FSM
designers actually think about this code.

**Against.** `max := rules { … }` is weak. Bluespec rules are *concurrent*,
not prioritized, so the precedent half-misleads.

### 4.22 `always { when … }`

```virdant
always {
    when reset      { phase <= #Red  timer <= 8 }
    when timer == 0 { phase <= next_phase }
    else            { timer <= timer - 1 }
}

// expression form makes no sense:
// max := always { when a > b => a  else => b }
```

**Rationale.** Verilog's process block. "always: when reset…, when
timer == 0…" reads naturally and HDL people will feel at home.

**Against.** Carries process/sensitivity-list baggage. Unusable in expression
position, so it would need to be paired with a different expression-level
header (see 4.37).

### 4.23 `cascade { when … }`

```virdant
cascade {
    when reset      { phase <= #Red  timer <= 8 }
    when timer == 0 { phase <= next_phase }
    else            { timer <= timer - 1 }
}

max := cascade {
    when a > b => a
    else       => b
}
```

**Rationale.** Names the hardware it generates (a cascade of 2:1 muxes).
English word, rarely a signal name, and it quietly teaches the reader
"priority, not parallel". Works in both positions.

**Against.** Seven letters. Slightly exotic as a keyword. Software readers
will not know the hardware meaning.

### 4.24 `pmux` / `prio { when … }`

```virdant
pmux {
    when reset      { phase <= #Red  timer <= 8 }
    when timer == 0 { phase <= next_phase }
    else            { timer <= timer - 1 }
}

max := pmux {
    when a > b => a
    else       => b
}
```

**Rationale.** "Priority mux", the engineer's name for the structure. Precise.
Four letters.

**Against.** Jargon; opaque to anyone not already a hardware person. `prio` is
a common signal-name abbreviation.

### 4.25 `which { when … }`

```virdant
which {
    when reset      { phase <= #Red  timer <= 8 }
    when timer == 0 { phase <= next_phase }
    else            { timer <= timer - 1 }
}

max := which {
    when a > b => a
    else       => b
}
```

**Rationale.** "max is which? — when a > b, a; else b." Short, English, oddly
satisfying in expression position. Five letters.

**Against.** Slightly weird at statement level ("which: when reset, drive
phase"). No precedent.

### 4.26 `whichever { when … }`

```virdant
whichever {
    when reset      { phase <= #Red  timer <= 8 }
    when timer == 0 { phase <= next_phase }
    else            { timer <= timer - 1 }
}

max := whichever {
    when a > b => a
    else       => b
}
```

**Rationale.** Same idea as `which`, more explicit.

**Against.** Nine letters. Too long.

### 4.27 `handle { when … }`

```virdant
handle {
    when reset      { phase <= #Red  timer <= 8 }
    when timer == 0 { phase <= next_phase }
    else            { timer <= timer - 1 }
}

// expression form reads poorly:
// max := handle { when a > b => a  else => b }
```

**Rationale.** Natural for event-ish FSM code ("handle: when reset…").

**Against.** Bad as an expression. Six letters.

### 4.28 `resolve { when … }`

```virdant
resolve {
    when reset      { phase <= #Red  timer <= 8 }
    when timer == 0 { phase <= next_phase }
    else            { timer <= timer - 1 }
}

max := resolve {
    when a > b => a
    else       => b
}
```

**Rationale.** VHDL has "resolution functions", so it is half-familiar. Reads
acceptably in both positions.

**Against.** Seven letters. Resolution functions resolve *multiple drivers*,
which is a different thing; the precedent is loose.

### 4.29 `dispatch { when … }`

```virdant
dispatch {
    when reset      { phase <= #Red  timer <= 8 }
    when timer == 0 { phase <= next_phase }
    else            { timer <= timer - 1 }
}

max := dispatch {
    when a > b => a
    else       => b
}
```

**Rationale.** Clear meaning.

**Against.** Eight letters. Collides with CPU-pipeline vocabulary
(`dispatch_valid`, dispatch stage). Avoid.

### 4.30 `consider { when … }`

```virdant
consider {
    when reset      { phase <= #Red  timer <= 8 }
    when timer == 0 { phase <= next_phase }
    else            { timer <= timer - 1 }
}

max := consider {
    when a > b => a
    else       => b
}
```

**Rationale.** Very natural English ("consider: when…").

**Against.** Eight letters. Long for something that appears constantly.

### 4.31 `try { when … }`

```virdant
try {
    when reset      { phase <= #Red  timer <= 8 }
    when timer == 0 { phase <= next_phase }
    else            { timer <= timer - 1 }
}

max := try {
    when a > b => a
    else       => b
}
```

**Rationale.** Implies sequential attempts, which correctly suggests priority.
Three letters.

**Against.** Exception-handling connotations are hard to shake. Avoid.

### 4.32 `find { when … }`

```virdant
find {
    when reset      { phase <= #Red  timer <= 8 }
    when timer == 0 { phase <= next_phase }
    else            { timer <= timer - 1 }
}

max := find {
    when a > b => a
    else       => b
}
```

**Rationale.** `Array.find` semantics: return the first match. Works in
expression position. Four letters.

**Against.** Odd in statement position ("find: when reset, drive phase").

### 4.33 `do { when … }`

```virdant
do {
    when reset      { phase <= #Red  timer <= 8 }
    when timer == 0 { phase <= next_phase }
    else            { timer <= timer - 1 }
}

max := do {
    when a > b => a
    else       => b
}
```

**Rationale.** Shortest option. "do: when reset…" reads fine at statement
level.

**Against.** `max := do { … }` is flat. `do` is a loop keyword in C-family
languages and a monad keyword in Haskell; both are wrong associations.

### 4.34 `choice { when … }`

```virdant
choice {
    when reset      { phase <= #Red  timer <= 8 }
    when timer == 0 { phase <= next_phase }
    else            { timer <= timer - 1 }
}

max := choice {
    when a > b => a
    else       => b
}
```

**Rationale.** The noun form of `choose`. Reads better than `choose` on the
right-hand side of `:=` ("max is the choice: when…").

**Against.** Slightly worse as a statement ("choice: drive phase"). Six
letters.

### 4.35 `value { when … }`

```virdant
// statement-level makes no sense:
// value { when reset { phase <= #Red } }

max := value {
    when a > b => a
    else       => b
}
```

**Rationale.** Expression-only; says "here is a value".

**Against.** Says nothing about priority or conditions. Unusable at statement
level. Avoid.

### 4.36 `table { when … }`

```virdant
table {
    when reset      { phase <= #Red  timer <= 8 }
    when timer == 0 { phase <= next_phase }
    else            { timer <= timer - 1 }
}

max := table {
    when a > b => a
    else       => b
}
```

**Rationale.** Evokes a truth table.

**Against.** A truth table implies *parallel*, mutually exclusive rows; this
is a priority chain. Misleading. Avoid.

### 4.37 Two-word header: `select first { when … }`

```virdant
select first {
    when reset      { phase <= #Red  timer <= 8 }
    when timer == 0 { phase <= next_phase }
    else            { timer <= timer - 1 }
}

max := select first {
    when a > b => a
    else       => b
}
```

**Rationale.** Precise about semantics.

**Against.** Verbose; clunky in expression position. Two keywords for one
construct. Avoid.

### 4.38 Different headers for statement and expression position

```virdant
// statement: a verb or process word
always {
    when reset      { phase <= #Red  timer <= 8 }
    when timer == 0 { phase <= next_phase }
    else            { timer <= timer - 1 }
}

// expression: a noun or question word
max := which {
    when a > b => a
    else       => b
}
```

**Rationale.** The two forms already differ in content (driver blocks vs
`=> expr`), so a verb at statement level and a noun/question word at
expression level is defensible. Lets `always` or `handle` be used where they
read well without forcing them into expression position.

**Against.** Two keywords to teach. The direct-chaining rule is easier to
explain when both forms look identical. Only worth it if no single word fits
both positions.

---

## Part 5 — Combinations worth considering

### 5.1 `choose { when … }` + guards on `match` arms

```virdant
choose {
    when reset      { phase <= #Red  timer <= 8 }
    when timer == 0 { phase <= next_phase }
    else            { timer <= timer - 1 }
}

max := choose {
    when a > b => a
    else       => b
}

match cmd {
    case @Get(addr) when addr == 0 { hit := true }
    case @Get(addr)                { hit := lookup(addr) }
    case @Put(addr, data)          { hit := false }
}

value := match cmd {
    case @Get(addr) when addr == 0 => 0
    case @Get(addr)                => mem[addr]
    case @Put(addr, data)          => data
}
```

**Rationale.** Once `when` is the condition keyword, it is the natural guard
keyword too (`case pat when cond`), which is also OCaml's and Scala's choice.
Now the whole language has exactly one word for "under this condition" and one
word for "matching this pattern", and `choose { when c … }` is definitionally
`match { case _ when c … }`. Removes the main reason people nest `when` inside
`match` arms.

### 5.2 `switch { when … }` + guards

Same as 5.1 with `switch` as the header. Choose this if you want the header to
carry a precedent (Go) at the cost of coexisting with `match`.

### 5.3 `match { when … }` + `match x { case … }`

Same as 4.17. One header, two arm keywords, no new tokens. Choose this if
minimising the keyword count matters more than the "subject-less `match`"
surprise.

---

## Summary ranking

Across everything above, with the stated goals in mind:

1. **`choose { when … }`** (4.1) — natural in both positions, no collisions,
   no misleading hardware connotation. Pair with guards (5.1).
2. **`switch { when … }`** (4.20) — best if you want a word readers will
   guess correctly on first sight, via Go's tagless `switch`.
3. **`cascade { when … }`** (4.23) — best if you want the keyword to teach the
   semantics (priority, not parallel).
4. **`match { when … }`** (4.17) — best if you want zero new keywords.
5. **`which { when … }`** (4.25) — best if the expression form matters most.
6. **`cond { when … }`** (4.2) — best if you want the PL-canonical name.
7. **`when { ? … }`** (2.16) — best if you are willing to spend a symbol.
8. **`when { at … }`** / **`when { given … }`** (2.3, 2.7) — smallest diff,
   if the header must stay `when`.
9. **`|` arm bars** (2.18) — dark horse; worth a prototype if a larger visual
   change is on the table.

Rejected outright: `with` on `match` arms (1.1), bare adjacent `when` chains
(3.4), `once`, `iff`, `branch`, `order`, `test`, `table`, `value`, `try`,
`dispatch`, `select first`.


# Personal Ideas

```virdant
select {
    if <booleanexpr> => <expr>
    if <booleanexpr> => <expr>
    ...
}
select if <subjectexpr> {
    is <pattern> => <expr>
    is <pattern> => <expr>
    ...
}
```



```virdant
if {
    when <booleanexpr> => <expr>
    when <booleanexpr> => <expr>
    ...
}
if <subjectexpr> {
    is <pattern> => <expr>
    is <pattern> => <expr>
    ...
}
```


```virdant
if {
    when <booleanexpr> => <expr>
    when <booleanexpr> => <expr>
    ...
}
if <subjectexpr> {
    is <pattern> => <expr>
    is <pattern> => <expr>
    ...
}
```

