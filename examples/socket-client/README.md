# Example: socket-client

Connects to Smart Eye Pro via either TCP or UDP and prints some parts of the
received output data packets.

```
> cargo run --example socket-client TCP
Connecting to TCP (hostname=localhost, port=5002)
FrameNumber = 42529
CameraPositions = [Point3D(0.2682732323511141, -0.1777000457370971, 0.5269263822146691), Point3D(-0.2036303979326044, -0.2203994197774216, 0.5508584634972726)]
----
[...]
```
