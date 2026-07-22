# RTCP Project (RTC Proxy) (formerly TURN proxy)

![GitHub License](https://img.shields.io/github/license/Urtyom-Alyanov/turn-proxy?style=for-the-badge&logo=gplv3&logoColor=FFFFFF)
![GitHub repo size](https://img.shields.io/github/repo-size/Urtyom-Alyanov/turn-proxy?style=for-the-badge&logo=github&logoColor=FFFFFF)
![GitHub top language](https://img.shields.io/github/languages/top/Urtyom-Alyanov/turn-proxy?style=for-the-badge&logo=rust&color=FF8000&logoColor=FFFFFF)
![GitHub Actions Workflow Status](https://img.shields.io/github/actions/workflow/status/Urtyom-Alyanov/turn-proxy/check.yml?style=for-the-badge&label=Checks)
![GitHub Actions Workflow Status](https://img.shields.io/github/actions/workflow/status/Urtyom-Alyanov/turn-proxy/build.yml?style=for-the-badge&label=Builds)
![Last Commit](https://img.shields.io/github/last-commit/Urtyom-Alyanov/turn-proxy?style=for-the-badge&logo=git&logoColor=FFFFFF)

[Читай меня на русском](./README_RU.md)

This project implements the WebRTC protocol to exchange **arbitrary information** over it. It is an evolution of the TURN proxy project, which simply encrypted UDP packets using DTLS and wrapped them in a TURN header.

The previous approach is already being detected by VK systems (the primary provider). Furthermore, while the previous project was reasonably well-written, it eventually hit its architectural "limit." It was literally my first project in Rust; now that my skills have improved, I see that the architecture needs a complete overhaul.

> [!IMPORTANT]
> I am writing this project primarily to study networking and "low-level" programming.
> Consequently, it does not have a single specific use-case and can be applied in any situation.

## Related Projects

- [olcrtc](https://github.com/openlibrecommunity/olcrtc) - This project is largely experimental, and many ideas will be drawn from it. It essentially implements what I wanted to achieve in the previous project. Licensed under WTFPL, written in Go. I recommend using it as a replacement for the obsolete `vk-turn-proxy` and `TURN proxy`.
- [Good TURN (vk-turn-proxy)](https://github.com/cacggghp/vk-turn-proxy) - A somewhat abandoned project, but essentially the first popular project in the region to popularize this method. Another source of inspiration. Written in Go, licensed under GPL-3.0. This repo also lists other projects that partially influenced this one.

## Why Rust

Rust offers several advantages:

- **Zero-cost abstractions** for almost everything. Due to the lack of a Garbage Collector (GC), the application can be paged out to swap, potentially occupying 0 bytes in physical RAM when idle.
- **"Fearless concurrency"**: the language's core mechanisms (ownership and borrowing) allow for the safe use of coroutines or OS threads without data races.
- **Binary performance**: Since Rust is built on top of LLVM, which has an excellent optimizer, and Rust's own rules provide more information and "certainty" to the LLVM optimizer, the resulting binary is highly optimized and fast.

However, there is one major disadvantage compared to Go: the relative immaturity of the WebRTC libraries themselves.

## Project Architecture

For development simplicity, the project is divided into several crates and sub-projects:

- `/providers/`: Contains server providers through which the connection is established.
- `/transports/`: Contains "filters" that transform your traffic (encryption, obfuscation, etc.).
- `/lib/`: The "bare" core of the project, provided as a library for clients and other projects.
- `/client/`: The CLI client itself, which combines both server and client functionality and includes its configuration.

### Licensing

The core library (`/lib/`) and related crates (`/providers/`, `/transports/`) are licensed under the [MPL-2.0 (Mozilla Public License 2.0)](./lib/LICENSE). The client is licensed under the [GNU AGPL-3.0 (GNU Affero General Public License 3.0)](./client/app/LICENSE).

This choice was made to preserve the rights to modification, use for any purpose, and other freedoms, regardless of any forks.

