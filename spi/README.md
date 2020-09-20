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
2. The Component drives the READY line low
3. The Component waits until the GO line is low
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

* [x] Work on Enc/Logic as a crate
* [ ] Get Enc/Logic working over TCP (SPI simulator)
    * [x] Low Level Interface
    * [x] High Level Interface
* Struct or Trait for HL interface?
* What to do for storage?
* [ ] Get Physical layer working (embedded-hal?)
* [ ] Get it working on two actual physical nrf52840s

# Thoughts on Network Layers


```
6. Application:  [ Application Data                                   ]
5. Routing:      [ Routing                                            ]
4. Client:       [ Anachro Session State                              ]
3. ClientIo:     [ Anachro ICD Data Types - Anachro Protocol          ]
2. Enc/Logic:    [ Serde/Postcard Encoded Data                        ]
1. Enc     :     [ ?COBS Encoded Data?   ][    State Logic            ] **** first
0. Physical:     [ SPI - COPI, CIPO, SCK ][ GPIO - READY ][ GPIO - GO ] **** second

TODO: Do these match?


6: [ ?????? ]           User Applications
       ^
       v
5: [ router ]           Logical Topics/App Protocol
       ^
       v
4: [ struct ]           (Client)
       ^
       v
3: [ trait  ]           (ClientIo)
       ^
       v
2: [ struct ]           (EncLogicHLComponent)
       ^
       v
1: [ trait  ]           (EncLogicLLComponent)
       ^
       v
0: [ struct ]           (TCPSpiClient, concrete protocol layer)
```
