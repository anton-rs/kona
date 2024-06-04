# `kona-host`

The host binary's primary role is to serve the client program responses to requests over the [Preimage Oracle ABI][preimage-spec].

## Modes

| Mode     | Description                                                                                                                                                                                            |
| -------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `server` | Starts with the preimage server only, expecting the client program to have been invoked by the host process. This mode is particularly purposed to be activated by the FPVM running the client program |
| `native` | Starts both the preimage oracle and client program in a native process, bypassing the verifiable FPVM environment. This mode is useful for upfront witness generation as well as testing.              |
