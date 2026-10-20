# Initialization
I need a way to initialize `reg`s on FPGAs to an initial value.

## Syntax
```virdant
mod Top {
    incoming clock : Clock

    reg counter : Word[3] on clock {
        initial counter <= 7

        when {
            case it > 0 {
                it <= it - 1
            }
        }
    }
}
```

This terns into Verilog:

```verilog
module Top;
    input clock;

    reg [2:0] counter;

    initial begin
        counter = 0
    end

    // ... @posedge(clock) logic
endmodule
```

## Notes
The initial expression must be a constant.
