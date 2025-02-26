# sb-crank-feed-rust

A Rust script to crank a Switchboard feed on Solana's Devnet.

## Usage

```bash
cargo run -- --keypair ~/my-wonderful-keypair.json
```

## Notes

- The keypair path is optional. If not provided, the script will look for a keypair at `~/.config/solana/id.json`.
- The script uses the devnet RPC URL and default Switchboard Queue.
