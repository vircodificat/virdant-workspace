# Default Drivers
Allow a new statement written:

Path "?=" Expr

the new `?=` operator is called a default driver.
It is continuous, like `:=`.
It can ONLY appear inside of `platform` defs.
It should be allowed grammatically in moddefs, but it is an error DefaultDriverNotInPlatform "?= may only appear in a platform"

## Purpose
Currently, a moddef which has a `for` clause *must* drive all ports of the associated platform.
This is onerous, and for many applications, you don't care about driving all output pins.

To this end, you may write a default driver, such as `tx ?= true` to give a default value to the port
*unless* it is overridden by the top module.

The expression MUST be a constant expression.
Do not evaluate it -- just compile it to Verilog.

It is typechecked in the platform just as if it were in a moddef.

## Plan
Please write PLAN.md.
