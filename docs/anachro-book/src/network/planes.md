# Network Planes

The Anachro Protocol exposes four different "planes" of communication, each with differing reliablity and usage profiles. These planes include:

1. **The Control Plane**
    * This plane is required to be implemented by all Anachro devices.
    * This plane is used for establishing connections, and configuration of devices over the network.
    * This plane has acknowledged messages and sequence numbers for improved reliability.
2. **The Pub/Sub Plane**
    * This plane is optional for Anachro devices to implement.
    * This plane is used for periodic sending of messages, such as sensor readings
    * This plane has no acknowledgement or sequencing of messages, and should be considered (relatively) unreliable compared to other layers.
    * This plane is loosely inspired by [MQTT-SN](http://www.steves-internet-guide.com/mqtt-sn/) with a QoS level of 0.
3. **The Object Store Plane**
    * This plane is optional for Anachro devices to implement.
    * This plane is for bulk message sending and storing for devices on the Anachro Network
    * This plane has acknowledgement of messages for improved reliability
    * This plane is loosely inspired by Redis or S3 when used as network caches
    * Note: As of 2020-10-10, this plane has not yet been implemented.
4. **The Mailbox Plane**
    * This plane is optional for Anachro devices to implement.
    * This plane is for sending small one-shot messages with guaranteed delivery between devices.
    * This plane has acknowledgement of reception and Ack/Nak of whether the message was placed in the mailbox of the other device (or if the mailbox was full)
    * This plane is typically coupled with the Object Store Plane to send references to bulk messages stored in the Object Store Plane.
    * Note: As of 2020-10-10, this plane has not yet been implemented.
