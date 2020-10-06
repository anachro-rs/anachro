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
Physical:     [ SPI - COPI, CIPO, SCK ][ GPIO - CSn ][  GPIO - GO  ] **** second
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
1. Enc     :     [ COBS Encoded Data     ][    State Logic            ] **** first
0. Physical:     [ SPI - COPI, CIPO, SCK ][ GPIO - CSn   ][ GPIO - GO ] **** second

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

```
* nrf52832 - Broker
  * USB-UART -> PC (client)
  * ESB      -> Lighting controller (client)
  * ESB      -> E-Paper Clock (client)
```

# 2020-09-26 notes

* I need to figure out how to "plug in" the TCP impls
  * Right now Client/Component has a good `ClientIo` trait, I don't think the arb has that
  * How can I trait-ify the client impl?
    * I need some kind of "inversion of control" - the server should probably own an IO provider
    * The client might want to own it, but I actually DON'T want that for the arbitrator, because it might have multiple IO transports, like ESB, SPI, UART, ETH, etc. Unless I expect people to impl their own wrapper struct that contains all of these? That might be possible, but for now, just passing in the ClientIO might just be easier.
    * I wonder if it would make lifetimes easier to just own the ClientIo. Maybe an experiment for another day.
* Once I have something like a `ServerIo` trait, I can provide an impl for TCP SPI
* Once that works, I should start impl'ing the SPI arbitrator for nrf52 (and maybe embedded-hal) SPI interfaces
  * Probably SPI controller can be embedded-hal, and get nrf52 for free
  * There aren't any SPI peripheral traits, so that's probably going to be direct for now

* I think I want to move the Client connection inside of the broker. This would clean some stuff up, but I can think of these problems:
  * I pretty much have to make this `dyn Trait`, if I want the clients to have different types/buffer sizes
