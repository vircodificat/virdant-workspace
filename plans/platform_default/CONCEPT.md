# Concept: Platform Defaults

* In platform defs, allow an optional `default <EXPR>` syntax after `outgoing` ports.
* `<EXPR>` must be a constant expression

* When a top module with a `for` clause is missing an `outgoing` port, it is no longer an error
* Instead, a synthetic `outgoing` port is created and given a synthetic driver which drives the value given by `<EXPR>`
* The expression is to be lowered to Verilog as an expression; it does not need to be evaluated by the compiler.

* When a top module with a `for` clause is missing an `incoming` port, it is no longer an erorr.
* Instead, a synthetic `incoming` port is created and treated as if it were marked `unused`, so no warning is generated.

## Motivation

A Virdant platform declares `incoming` and `outgoing` ports.
Every module bound to the platform via a `for` clause must currently declare *all* of them.
Otherwise, Virdant reports a `PlatformMissingPort` error.

This is unnecessarily rigid.
Asking every module to drive them with a repetitive `it := true` is boilerplate.
Conversely, an `incoming` port that a module simply doesn't care about must still be declared,
even though it will be marked `unused`.

## Proposal: `default <EXPR>` on platform `outgoing` ports

Allow an optional `default <EXPR>` clause on `outgoing` port declarations
in a `platform` definition.

```virdant
platform SpiFlash {
    incoming clock     : Clock
    incoming read_addr : Valid[Word[24]]
    outgoing read_data : Valid[Word[32]]

    outgoing device_ready : Bit        default true
    outgoing spi_ck       : Bit        default false
    outgoing spi_cs       : Bit        default true
    outgoing spi_mosi     : Bit        default false

    incoming spi_miso : Bit
}
```

`<EXPR>` must be a constant expression
(literal, enum variant `#Foo`, `@Invalid()`, `@Valid(n)`, struct literal `${...}`, or any expression composed of those).
The compiler does **not** evaluate or constant-fold it; it is lowered verbatim as a Verilog expression.

### Grammar
The syntactic change belongs on the outgoing port statement itself,
not on the platform-specific top-level alternative.

Both PlatformStmt and ModDefStmtComponent (which is used by plain mod definitions)
already share a common rule for outgoing declarations.
That rule gains an optional default <EXPR> clause:

```
  // In the rule for outlet/outgoing port statements:
  OutgoingStmt: AstNodeId = {
      "outgoing" <ident:Ident> ":" <typ:Type> ["default" <expr:Expr>] => { … },
  }
```

`default` becomes a keyword.

Because default is part of the shared OutgoingStmt rule, it is grammatically accepted in both platform and mod definitions.
A separate semantic check (in the analyser, not the parser)
reports an error when a plain mod definition uses default on an outgoing port:

│ DefaultOutsidePlatformPort: `default` is only allowed on `outgoing` ports in a platform definition.


## Behaviour: synthesised ports

When a module with a `for` clause is checked against its platform (`check_platform_ports_for`),
a missing platform port is no longer an error.
Instead:

### Missing `outgoing` → synthesised port + driver

A synthetic `outgoing` port is created for the module,
together with a synthetic continuous driver that assigns the platform's default expression:

```virdant
// Platform
platform MyPlatform {
    outgoing spi_ck : Bit default false
    // ...
}

// Module — does not declare spi_ck
mod MySpiFlash for MyPlatform {
    // … other ports …

    // Synthesised as if the user had written:
    //   outgoing spi_ck : Bit {
    //       it := false
    //   }
}
```

If the platform declares no `default` for that outgoing port, the
existing `PlatformMissingPort` error is still emitted.

### Missing `incoming` → synthesised unused port

A synthetic `incoming` port is created for the module.
The port is internally marked as `unused`,
so the compiler does **not** emit an "unused port" warning.
(An incoming port cannot have a default — there is nothing to drive from the module's side.)

```virdant
// Platform
platform SpiFlash {
    incoming spi_miso : Bit
}

// Module — does not declare spi_miso
for SpiFlash
mod MySpiFlash {
    // … other ports …

    // Synthesised as if the user had written:
    // incoming spi_miso : Bit
    // unused spi_miso
}
```

## Non-goals

- No `default` on `incoming` ports (incoming defaults would mean
  "substitute this value when the FPGA input is unconnected", which is a
  hardware concept outside the compiler's scope).
- The compiler does not evaluate the default expression; it is passed
  through to Verilog code generation as an opaque expression.

## Notes
- How might this feature interact with pullups?
