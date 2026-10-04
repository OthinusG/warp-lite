# Superseded Enrolled-Device Gateway

The device gateway is [historical](https://github.com/OthinusG/warpai/blob/47a2a5a/specs/agent-communication-v2/legacy-machine-collaboration/REMOTE-GATEWAY.md).
Its runtime has been removed. The active companion uses bounded protobuf over
system SSH to a private remote account service; see [API.md](API.md) and
[TECH.md](TECH.md). No public relay, enrollment or desktop credentials are used.
