# TODO — Examples and Open Questions

**Last updated**: October 15, 2024  
**Status**: ✅ **28/28 Examples Complete** | 🟡 **1 Design Question + 1 Example Correction Needed**

See `FINAL_SUMMARY.md` for comprehensive overview.

---

## ✅ All Examples Complete

All 28 examples covering the Virdant language have been created and validated:

### Core Algorithm (001-016)
- Simple modules, literals, operators
- Error cases (async/register, mixed clocks, mixed domains)
- Cross-module instantiation
- Multi-clock modules
- Unconstrained ports
- @unsafe_clocks
- Pure constant modules

### Language Features (017-028)
- Bit slicing, indexing
- word() concatenation
- zext/sext extension
- asclock() type conversion
- dontcare free variables (**needs correction**, see below)
- mux() ternary
- Statement-level when/match
- Bit shifting patterns
- Union payloads
- Enum cast
- Forwarding blocks
- Hole expressions (`?`) free variables

---

## ⚠️ Example Requiring Correction

### **Example 021 (dontcare) - INCORRECT**

**Issue**: Example 021 shows `dontcare` with `Fixed(Constant)` domain, but this is **wrong**.

**Correct behavior** (discovered in Example 028):
- `dontcare` is a **free variable** (no Fixed constraint)
- It unifies with any domain through equality constraints
- Can be used in Async contexts: `wire w : Bit async { it := dontcare }` ✓
- Same domain behavior as holes (`?`), but no diagnostic

**What needs updating in Example 021:**
1. Remove `Fixed(dontcare, Constant)` from Pass 1
2. Show dontcare as free variable in Pass 2
3. Demonstrate dontcare unifying with both Sync and Async contexts
4. Update Notes to explain distinction from literals:
   - Literals: `Fixed(Constant)` - explicit constant values
   - dontcare: Free variable - compiler-chosen flexible value
   - Holes: Free variable - user-written flexible value

**Priority**: Should be corrected before implementation to avoid confusion.

---

## 🟡 Remaining Open Question

### **Socket Domain Rules** (CRITICAL for socket support)

**Status**: Open design question in PLANv2.md §TODO

**Issue**: How do socket channels (cosi/soci) interact with domain analysis?

Sockets bundle multiple signals with directions (client-out-server-in vs server-out-client-in).

**Key questions**:
1. Does each socket channel have an independent domain?
2. When using bulk connect (`:=:`), must all channels have compatible domains?
3. Can a socket have channels on different clock domains?
4. Socket driver blocks (`it.ready := ...`) - how are domains assigned to it.field?
5. Are there special rules for handshaking signals (ready/valid) that might cross domains?

**Example scenario needing design decision**:
```virdant
socket Pipe {
    cosi valid : Bit
    cosi data  : Word[8]
    soci ready : Bit
}

mod Producer {
    incoming clk_a : Clock
    incoming clk_b : Clock
    
    client socket out of Pipe {
        // Can the channels be on different clocks?
        it.valid := reg_on_clk_a  // Sync(clk_a)
        it.data  := reg_on_clk_b  // Sync(clk_b)
        // What domain does out.ready have?
    }
}
```

**Possible design approaches**:

1. **Independent channel domains** (most flexible):
   - Each socket channel is an independent signal with its own domain
   - Bulk connect (`:=:`) requires compatible domains channel-by-channel
   - Socket driver blocks constrain each `it.field` independently
   - Allows mixing clocks within a socket (handshaking across domains)

2. **Unified socket domain** (simpler, more restrictive):
   - All channels in a socket must have the same domain
   - Socket as a whole gets a single domain assignment
   - Bulk connect requires socket-level domain compatibility
   - Cross-clock handshaking requires explicit async annotations

3. **Hybrid** (directional):
   - cosi channels must share a domain (client's responsibility)
   - soci channels must share a domain (server's responsibility)
   - The two directions can differ (e.g., async request, sync response)

**Recommendation**: Defer socket domain rules decision until:
- Initial clock domain analysis implementation is complete
- Real socket usage patterns from existing code can inform the design
- Performance/usability tradeoffs can be evaluated empirically

**Implementation strategy**: 
- Sockets can initially be treated as errors ("socket domain analysis not yet implemented")
- Add socket support in a follow-up phase once design is settled
- Most code uses simple socket patterns that will work under any reasonable design

---

## Algorithm Updates Required

Based on the 27 examples, the following algorithm updates are needed:

### Critical (Must Fix)

✅ **Fixed literal domain** (Example 016)  
- Change: `Fixed(literal, Constant)` instead of no constraints

✅ **asclock() primitive** (Example 020)  
- Add: `Fixed(asclock_result, Clock)` type conversion rule

❌ **dontcare is FREE VARIABLE** (Example 021 correction needed)  
- Change: NO Fixed constraint for dontcare (it's a free variable)
- Can unify with any domain including Async

❌ **Holes are FREE VARIABLE** (Example 028)  
- Change: NO Fixed constraint for holes (they're free variables)
- Can unify with any domain including Async

✅ **Statement-level when/match** (Example 023)  
- Document: Independent per-target constraint generation

### Important (Should Add)

✅ **word() concatenation** (Example 018)  
✅ **Union constructors** (Example 025)  
✅ **Unary operations** (Examples 017, 019, 026)  

See `FINAL_SUMMARY.md` Section "Algorithm Updates Required" for detailed descriptions.

---

## Testing Strategy

Each of the 27 examples can become a unit test:

**Error case tests** (should fail with specific error):
- 005, 006, 007, 008, 009, 012

**Success case tests** (should pass analysis):
- 001, 002, 003, 004, 010, 011, 013, 014, 015, 016, 017, 018, 019, 020, 021, 022, 023, 024, 025, 026, 027

**Test implementation**:
1. Extract Virdant code from each example markdown
2. Run domain analysis
3. Verify expected result (pass or specific error)
4. Check computed domains match expected domains from example

**Coverage**: 22 success tests + 6 error tests = 28 total tests covering all major language features

**Note**: Example 021 needs correction before use as test case.

---

## Documentation Status

### Complete ✅
- ALGORITHM.md (needs updates from examples)
- PLANv2.md (needs additions from examples)
- REQUIREMENTS.md (complete)
- EXTRA.md (design evolution notes)
- examples/PREFACE.md (overview)
- examples/SUMMARY.md (first batch summary)
- examples/FINAL_SUMMARY.md (comprehensive summary)
- examples/001-027 (all examples with Notes)

### Needs Minor Updates
- ALGORITHM.md: Add literal fix, primitives, statement-level when, word(), dontcare, unions
- PLANv2.md: Add statement-level distinctions, union construction, socket TODO remains

### Ready for Implementation
- All design questions resolved (except sockets)
- All language features analyzed
- Algorithm validated across 27 diverse examples
- Test suite outline ready

---

## Next Actions

1. ✅ Review FINAL_SUMMARY.md (updated with holes/dontcare discovery)
2. ❌ **Correct Example 021 (dontcare)** - free variable, not Constant
3. ⏳ Apply algorithm updates to ALGORITHM.md and PLANv2.md (or defer until implementation)
4. 🟡 Decide when to resolve socket domain rules (now vs. during/after implementation)
5. ⏳ Begin implementation with 28 examples as reference (after Example 021 correction)
6. ⏳ Add socket support once design is settled

---

**Example suite complete!** 🎉

28 examples totaling ~15,000 lines covering all major language constructs and edge cases.

**Important**: Example 021 needs correction (dontcare should be free variable, not Constant).
