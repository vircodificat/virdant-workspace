# Initial
Date: 2026-10-20
Status: Completed

Adds register initialization to Virdant — the ability to specify an initial
value for FPGA `reg` declarations instead of the default zero. The `initial`
clause on a reg block accepts a constant expression and emits a Verilog
`initial` block in codegen.
