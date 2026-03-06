# Blockchain Service

This service provides a simple, local blockchain simulation with an API for interacting with it. You can start the chain, send tokens between accounts, swap tokens using an Automated Market Maker (AMM), and query for block information.

## Getting Started

[Install Rust](https://rust-lang.org/tools/install/) if you haven't yet.

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Build the local blockchain in the `services` directory.

```bash
cd services
cargo build
```

Run the blockchain in the background or in its own terminal.

```bash
cargo run -- --start-chain --num-accounts 20
```

Learn more about available commands.

```bash
cargo run -- --help
```

## CLI Reference

### Starting the Chain

- `--start-chain`: Starts the blockchain simulation server. This process will run continuously, producing new blocks every second and listening for API requests on `http://127.0.0.1:3000`.

- `--num-accounts <NUMBER>`: Used in conjunction with `--start-chain` to specify how many accounts to create. If not provided, it defaults to 10.

### Interacting with the Chain

To interact with the running chain, you'll need to open a separate terminal.

- `--send <SENDER_ID> <RECEIVER_ID> <AMOUNT> <TOKEN>`: Sends a specified `AMOUNT` of a `TOKEN` from a sender to a receiver.
    - `<SENDER_ID>`: The account ID of the sender.
    - `<RECEIVER_ID>`: The account ID of the receiver.
    - `<AMOUNT>`: The amount of the token to send (e.g., `10.5`).
    - `<TOKEN>`: The type of token to send. Can be `USDC`, `USDT`, `ETH`, `BTC`, `NEX`, `DOGE`, or `HYPE`.

- `--swap <ACCOUNT_ID> <IN_TOKEN> <OUT_TOKEN> <AMOUNT>`: Swaps an `AMOUNT` of an input token for an output token for a given account.
    - `<ACCOUNT_ID>`: The account ID performing the swap.
    - `<IN_TOKEN>`: The token to trade.
    - `<OUT_TOKEN>`: The token to receive.
    - `<AMOUNT>`: The amount of the input token to swap.

- `--block [NUMBER]`: Fetches and displays the most recent blocks.
    - `[NUMBER]` (optional): The number of recent blocks to fetch. If omitted, it defaults to 10.

**Examples:**

Send 50.5 NEX from account 0 to account 1:
```bash
cargo run -- --send 0 1 50.5 NEX
```

Swap 10 USDC for USDT from account 2:
```bash
cargo run -- --swap 2 USDC USDT 10
```

Get the last 5 blocks:
```bash
cargo run -- --block 5
```

Get the last 10 blocks (default):
```bash
cargo run -- --block
```

## API Reference

### Interacting with the Chain via HTTP Requests

The blockchain server exposes a REST API on `http://127.0.0.1:3000` that allows you to interact with the chain programmatically. Below are the available endpoints:

#### Submit Transaction

**Endpoint:** `POST /transaction`  
**Content-Type:** `application/json`

Submit a transaction to the blockchain. The transaction will be added to the pending transactions queue and included in the next block.

**Transaction Types:**

1. **Send Transaction** - Transfer tokens between accounts:
```json
{
  "Send": {
    "from": 0,
    "to": 1,
    "token": "NEX",
    "amount": 50.5
  }
}
```

2. **Swap Transaction** - Exchange tokens via AMM:
```json
{
  "Swap": {
    "account": 2,
    "in_token": "USDC",
    "out_token": "USDT",
    "amount": 10.0
  }
}
```

**Example with curl:**

Send 50.5 NEX from account 0 to account 1:
```bash
curl -X POST http://127.0.0.1:3000/transaction \
  -H "Content-Type: application/json" \
  -d '{
    "Send": {
      "from": 0,
      "to": 1,
      "token": "NEX",
      "amount": 50.5
    }
  }'
```

Swap 10 USDC for USDT from account 2:
```bash
curl -X POST http://127.0.0.1:3000/transaction \
  -H "Content-Type: application/json" \
  -d '{
    "Swap": {
      "account": 2,
      "in_token": "USDC",
      "out_token": "USDT",
      "amount": 10.0
    }
  }'
```

#### Get Recent Blocks

**Endpoint:** `GET /blocks`  
**Query Parameters:**
- `n` (optional): Number of recent blocks to retrieve (defaults to all blocks)

**Response:** JSON array of block objects

**Example with curl:**

Get the last 5 blocks:
```bash
curl "http://127.0.0.1:3000/blocks?n=5"
```

Get all blocks:
```bash
curl "http://127.0.0.1:3000/blocks"
```

**Example Response:**
```json
[
  {
    "id": 42,
    "timestamp": 1699123456,
    "transactions": [
      {
        "Send": {
          "from": 0,
          "to": 1,
          "token": "NEX",
          "amount": 50.5
        }
      }
    ]
  }
]
```

#### Get Exchange Rate (Quote)

**Endpoint:** `GET /rate`  
**Query Parameters:**
- `in`: Input token symbol (e.g., `NEX`)
- `out`: Output token symbol (e.g., `ETH`)
- `amount` (optional): Input amount to quote (defaults to `1.0`)

**Response:** JSON object with the quoted output amount

**Example with curl:**

Get a quote for swapping 10 NEX to ETH:
```bash
curl "http://127.0.0.1:3000/rate?in=NEX&out=ETH&amount=10"
```

**Example Response:**
```json
{
  "amount_out": 9.0909090909
}
```

**Available Tokens:**
- `USDC`
- `USDT`
- `ETH`
- `BTC`
- `NEX` (native)
- `DOGE`
- `HYPE`
