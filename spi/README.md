# Notes on the SPI protocol

## Physical Components:

* COPI: Controller Out, Peripheral In
    * Line used to send data from the Component to the Arbitrator
* CIPO: Controller In, Peripheral Out
    * Line used to send data from the Arbitrator to the Component
* SCK: Serial Clock
    * Line used to clock data. Driven by the Component
* READY: Component Ready to Send/Receive
    * Digital IO. Driven low by the Component when ready to send/receive
* GO: Arbitrator Ready to Send/Receive
    * Digital IO. Driven low by the Arbitrator when ready to send/receive

## Misc Notes:

* Both sides shall always use an ORC of `0x00`.
* The Component shall operate at a speed <= the max speed of the Arbitrator
* TODO: How to read part of the message without dropping?

Do I want to just build this on top of a bidirectional BBQueue?

## Message Sequence

1. The Component is idle
2. The Component drives the GO line low
3. The Component waits until the READY line is low
4. The Component clocks 4 bytes:
    * The Component sends the length of the next message in bytes, or zero if no message
    * The Arbitrator sends the length of the next message in bytes, or zero if no message
    * TODO: Varints?
5. The Component decides whether to continue.
    * If not, the component releases the READY line high, and returns to state 1
    * If so, the component checks if the GO line is still low.
        * If not, the component releases the READY line high, and returns to state 1
        * If so, the component continues to step 6
6. The Component clocks up to `max(in_len, out_len)` bytes
    * Note: If any bytes are clocked by the Component, the Arbitrator will consider this message sent
7. The Component decides whether to continue.
    * If not, the component releases the READY line high, and returns to state 1.
    * If so, the component returns to state 3.

## Component Side (SPI Controller)

## Arbitrator Side (SPI Peripheral)



# Layer Diagram

```
Application:  [ Application Data                                   ]
????????:     [ Anachro Session State                              ]
????????:     [ Anachro ICD Data Types - Anachro Protocol          ]
????????:     [ Serde/Postcard Encoded Data                        ]
Enc/Logic:    [ COBS Encoded Data     ][    State Logic            ] **** first
Physical:     [ SPI - COPI, CIPO, SCK ][ GPIO - READY ][ GPIO - GO ] **** second
```

## 2020-09-19 Stream

* Work on Enc/Logic as a crate
* Get Enc/Logic working over TCP (SPI simulator)
* Get Physical layer working (embedded-hal?)
* Get it working on two actual physical nrf52840s
