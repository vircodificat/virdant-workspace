# Concept: conditional_syntax

We are going to change the syntax for conditionals in Virdant to look like this:

## If ... when / match ... with
```virdant
if {
    when <booleanexpr> => <expr>
    when <booleanexpr> => <expr>
    ...
}
match <subjectexpr> {
    with <pattern> => <expr>
    with <pattern> => <expr>
    ...
}
```


