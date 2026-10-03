# Superseded Enrolled-Device Routing

The device-routing design is [historical](legacy-machine-collaboration/NATIVE-REMOTE-ROUTING.md).
Its runtime has been removed; migrations preserve history and uncertain intent.
Current SSH same-project Agent communication follows [PLAN.md](PLAN.md),
[TECH.md](TECH.md) and [API.md](API.md). It reuses native MCP, Broker/Store and
system OpenSSH. File transfer and retained-session management are out of scope.
