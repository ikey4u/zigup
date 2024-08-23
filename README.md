# zigup

A [Zig](https://ziglang.org/) installer and manager.

## Installation

    cargo install --git https://github.com/ikey4u/zigup

## Usage

    zigup install [version]

    # use socks5 or http proxy to update
    zigup --proxy socks5://127.0.0.1:1080 install
    zigup --proxy http://127.0.0.1:1087 install
