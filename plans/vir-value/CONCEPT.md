# Concept: vir-value
I want a way, given a Virdant project, to consider the *types* in the project (both pub and non-pub) and to:

- encode a value (go from a value to a bit pattern)
- decode a bit pattern (go from a a bit pattern to a value).

```console
$ vir value encode @Valid(0) : Valid[Word[8]]
0b1000000000
$ vir value encode @Valid(0w8)
0b1000000000
$ vir value encode @Valid(0)
ERROR: Can't infer type
$ vir value decode 0b1000000000 Valid[Word[8]]
@Valid(0w8)
```

Names should be fully-qualified except the ones from the `builtin` package.
It should re-use the parsers in virdant packages `syntax`.


## Value subgrammar
```
   ValueExpr :=
         WordLit                          // 123, 0b1010_0101, 0xDEADw16, 100w32, 0b1010w8
       | BitLit                           // true | false
       | "@" Ident                        // payload-less ctor/enumerant: @Invalid
       | "@" Ident "(" ValueArgList? ")"  // ctor application: @Valid(0w8)
       | "$" "{" ValueAssignList "}"      // struct literal: ${Pair(a = 1, b = 2)}
       | "(" ValueExpr ")"
       | ValueExpr ":" Type               // ascription

   ValueArgList   := (ValueExpr ",")+ ValueExpr ","?
   ValueAssignList := (ValueAssign ",")+ ValueAssign ","?   // also empty
   ValueAssign    := Ident "=" ValueExpr
```
