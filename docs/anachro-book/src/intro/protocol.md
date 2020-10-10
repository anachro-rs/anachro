# The Anachro Protocol

The Anachro Network Protocol is a low complexity, frame oriented, transport agnostic messaging protocol that is designed to be usable over a variety of different physical layers, including SPI, UART, TCP, or wireless packet radios.

It is split into seven layers, allowing for re-use of higher layers when replacing the physical transport, or porting the Anachro protocol to new types of hardware. A longer discussion of the Network Protocol can be found in the [Network Chapter](../network/intro.md).

The Anachro Protocol exposes four different "planes" of communication, each with differing reliablity and usage profiles. These planes include a "Control" plane, a "Pub/Sub" plane, an "Object Store" plane, and a "Mailbox" plane. More information about the planes can be found in the [Network Chapter](../network/intro.md).
