# Concept: virt

I want to create a Virdant `platform` in the standard library `lib/virt.vir` called `Virt`.
`virt` is a "virtual FPGA".
It will have the following ports:

- clock (48 MHz)
- tx, rx UART pins
- SPI flash spi_ck, spi_cs, spi_di, spi_do

The actual board will be implemented with Verilator.
Take inspiration from virdant-riscv.

`vir upload` should actually create a binary in build rather than flash it to an actual FPGA with iceprog, etc.

The binary should be named `build/${TOP}` where `$TOP` is the name of the Top module.

The binary, written in Verilor and C++, should:


- Emulate UART over tx and rx, sending and receiving data to stdin/stdout
- Emulate a SPI flash device.
    - the device should be initially set to all 0xFF bytes.
    - however, you can specify a binary file with `--flash=<FILEPATH>` which may be up to 2^24 bytes
      and when a SPI Flash read command takes place (`READ 0x03`) it will begin sending back bytes from the given address


## Features
You know when `virt` is being targetted when you see `for virt::Virt` on the top module definition.


# Future Work
New ports:
- RGB LED: led_red led_green led_blue
- 4 Buttons: button_up button_down button_left button_right

- Rate-limit to the frequncy 48 MHz.
- If `--timing` is set, make sure it hits timing.
