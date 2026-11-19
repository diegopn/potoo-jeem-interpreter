# Security Policy

## Reporting a vulnerability

Do not publish vulnerability details in issues, pull requests, discussions, or public comments. If the private vulnerability reporting form is available on this repository's Security page, use it. Otherwise, use the [Potoo contact page](https://diegopn.github.io/#contato) only to request a private reporting channel. Do not include technical details or a proof of concept in that first message.

After arranging a private channel, include the affected commit, Rust version, operating system, reproduction steps, and impact. Remove credentials, personal information, and unrelated data.

For other project concerns, see [CONTRIBUTING.md](CONTRIBUTING.md) and [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).

## Execution scope

The library interprets the source forms listed in [REFERENCE-COVERAGE.md](REFERENCE-COVERAGE.md). The CLI reads one named source file and connects language input/output to standard input/output. It provides no language-level file access, and its default graphics host reports drawing as unavailable. The crate does not sandbox source programs.

The interpreter applies a step limit to executed statements and loop checks. That limit does not bound source size, expression depth, memory use, host I/O time, or behavior in a custom `Host` implementation. Input may block. A custom host can perform additional side effects, so callers must treat the host implementation as part of the execution trust boundary.

Records, pointer operations, file operations, most graphics commands, arbitrary library loading, and several built-in functions remain unavailable because the accessible references do not provide enough executable syntax or semantics. Do not treat this crate as a compatibility or isolation boundary for arbitrary programs.
