VIR := $(abspath virdant/target/release/vir)

.PHONY: test

test:
	$(MAKE) -C virdant test
	$(MAKE) -C virdant-riscv build VIR=$(VIR)
	$(MAKE) -C virdant-riscv build_tests
	$(MAKE) -C virdant-riscv scoreboard VIR=$(VIR)
