# The Anachro PC

The Anachro-PC Architecture is an extensible PC architecture intended to be composed of multiple individual microcontrollers. It is designed to be a teaching and hobbyist grade PC architecture, with a performance goal of approximating the capabilities of mid to late 1980's PCs such as the [IBM PC XT](https://en.wikipedia.org/wiki/IBM_Personal_Computer_XT).

Communication is managed by a single Arbitrator, which coordinates communication with standalone Cards, which each perform one dedicated PC function, such as the Main CPU, Keyboard Controller, Display Interface, Modem/Ethernet Interfaces, etc.

The Arbitrator in the Anachro PC Architecture fulfills approximately the role of the [Northbridge](https://en.wikipedia.org/wiki/Northbridge_\(computing\)) in modern x86/x86_64 PCs. It is also responsible for managing the main system memory.

The Anachro-PC Arbitrator and all cards communicate using the Anachro Network Protocol, used as a replacement for the use of PCI-E or dedicated Memory interfaces used in current day PCs. Cards are typically connected to the Arbitrator through the use of a hardware backplane, or direct wired connections.

A list of Anachro PC implementations can be found in the [Anachro PCs Chapter](../anachro-pcs/intro.md).
