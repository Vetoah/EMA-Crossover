# EMA-Crossover ![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)
This was a variation of the EMA crossover strategy that I built with Rust. It never made it out backtesting because it wasn't very profitable. Only when market conditions were perfect for this strategy was it profitable, but just barely.

The market data is coming from the Phemex crypto exchange. Here is there [API](https://github.com/phemex/phemex-api-docs/blob/master/Public-Contract-API-en.md)

## How to run
I do this through VS Code, so things may be different on your end. Would recommend installing [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer).
1. Install [cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html)
2. Clone this repo.
4. Open this folder in VS Code (the root, not 'src').
5. A target folder should automatically appear and begin building.
6. Run command: cargo run
7. See all of the losing trades. Cry.
