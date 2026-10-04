# The T Operating System

A very much work-in-progress OS & kernel for learning rust, x86-64 asm and C.

the current road map is:

- shell
- file system
- GUI
- own bootloader (that works with UEFI)

## Network Stack

Currently, the network stack supports TCP/IP and UDP/IP. It has a very basic implementation of TCP that doesn't handle congestion control, packet loss recovery, etc. Furthermore, it doesn't support IP fragmentation, though that would be pretty trivial to add. As of now, while it does support DHCP, DNS is not yet implemented, either.

## Environment

As Atomic{U,I}128 aren't yet completely stabilized in Rust, tOS has rolled it's own implementations! The project therefore depend on the Rust intrinsics to allow the LLVM backend to optimize the atomic operations. That means however, that the project can be a little sensitive to compiler versions. In my environment, I can compile using:

```
rustc 1.101.0-nightly (d080e7dff 2026-09-27)
binary: rustc
commit-hash: d080e7dff1b0fc54541545252818f8cccf995d05
commit-date: 2026-09-27
host: x86_64-unknown-linux-gnu
release: 1.101.0-nightly
LLVM version: 23.1.1
```
