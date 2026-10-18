# Platforms

Date: 2026-10-04

An in-language platform binding system for Virdant.
A top module declares which FPGA platform it implements using a `for` clause (`mod Top for ice40::IceSugar`),
and the module's ports must exactly match the platform's ports.
Replaces an earlier config-driven approach.
