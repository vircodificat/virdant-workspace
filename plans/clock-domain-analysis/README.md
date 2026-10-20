# Clock Domain Analysis

Date: 2026-10-15
Status: In Planning

Every signal in a Virdant design is assigned a clock domain:
`Sync(ComponentId)`, `Async`, `Clock`, `Reset`, or `Constant`.
Domain boundaries are tracked to detect illegal crossings and to drive code generation.
