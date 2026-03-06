use axum::{
    extract::{Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::sleep;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

/// Initializes and starts the blockchain simulation.
/// This function sets up the logger, creates the initial chain state,
/// and starts the block producer and the API server.
pub async fn start_chain(num_accounts: u32) {
    // This subscriber is used for logging events.
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default subscriber failed");

    // The chain state is created and wrapped in a thread-safe smart pointer.
    let chain = Chain::new(num_accounts);
    let shared_state = Arc::new(Mutex::new(chain.state));

    // The block producer runs in a separate asynchronous task.
    let block_producer_state = Arc::clone(&shared_state);
    tokio::spawn(async move {
        produce_blocks(block_producer_state).await;
    });

    // The API server is started to handle incoming requests.
    run_server(shared_state).await;
}

/// Represents the different types of tokens available in the chain.
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Token {
    USDC,
    USDT,
    ETH,
    BTC,
    NEX,
    DOGE,
    HYPE,
}

/// This implementation allows parsing a string into a Token.
/// It is case-insensitive and supports alternative names.
impl FromStr for Token {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "USDC" => Ok(Token::USDC),
            "USDT" => Ok(Token::USDT),
            "ETH" => Ok(Token::ETH),
            "BTC" => Ok(Token::BTC),
            "NEX" => Ok(Token::NEX),
            "DOGE" => Ok(Token::DOGE),
            "HYPE" => Ok(Token::HYPE),
            _ => Err(format!("'{}' is not a valid token", s)),
        }
    }
}

/// A type alias for account identifiers for clarity.
pub type AccountId = u32;

/// Defines the types of transactions that can be processed.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Transaction {
    /// A transaction to send a certain amount of a token from one account to another.
    Send {
        from: AccountId,
        to: AccountId,
        token: Token,
        amount: f64,
    },
    /// A transaction to swap one token for another through an AMM pool.
    Swap {
        account: AccountId,
        in_token: Token,
        out_token: Token,
        amount: f64,
    },
}

/// Represents a block in the blockchain.
/// Each block has a unique ID, a timestamp, and a list of transactions.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Block {
    pub id: u64,
    pub timestamp: u64,
    pub transactions: Vec<Transaction>,
}

/// Represents an Automated Market Maker (AMM) liquidity pool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmmPool {
    reserves: HashMap<Token, f64>,
}

impl AmmPool {
    /// Creates a new AMM pool with initial reserves for two tokens.
    fn new(token_a: Token, amount_a: f64, token_b: Token, amount_b: f64) -> Self {
        let mut reserves = HashMap::new();
        reserves.insert(token_a, amount_a);
        reserves.insert(token_b, amount_b);
        AmmPool { reserves }
    }
}

/// Creates a canonical key for a token pair to uniquely identify a liquidity pool.
/// The tokens are ordered to ensure that (A, B) and (B, A) produce the same key.
fn get_pool_key(token_a: Token, token_b: Token) -> (Token, Token) {
    if token_a < token_b {
        (token_a, token_b)
    } else {
        (token_b, token_a)
    }
}

/// Represents the entire state of the blockchain at a given moment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainState {
    /// A map of account IDs to their token balances.
    pub accounts: HashMap<AccountId, HashMap<Token, f64>>,
    /// A map of token pairs to their corresponding AMM pools.
    pub pools: HashMap<(Token, Token), AmmPool>,
    /// A vector of all blocks that have been produced.
    pub blocks: Vec<Block>,
    /// A list of transactions that have been received but not yet included in a block.
    pub pending_transactions: Vec<Transaction>,
    /// The ID to be assigned to the next block.
    next_block_id: u64,
}

/// The main chain structure, which holds the current state.
pub struct Chain {
    pub state: ChainState,
}

impl Chain {
    /// Creates a new chain with an initial state.
    /// It initializes accounts with an equal distribution of tokens and sets up the AMM pools.
    pub fn new(num_accounts: u32) -> Self {
        // Initialize the accounts and distribute the initial token supply.
        let mut accounts = HashMap::new();
        let tokens = [
            Token::USDC,
            Token::USDT,
            Token::ETH,
            Token::BTC,
            Token::NEX,
            Token::DOGE,
            Token::HYPE,
        ];
        let total_supply: f64 = 1_000_000.0;
        let amount_per_account = if num_accounts > 0 {
            total_supply / num_accounts as f64
        } else {
            0.0
        };

        for i in 0..num_accounts {
            let mut balances = HashMap::new();
            for &token in &tokens {
                balances.insert(token, amount_per_account);
            }
            accounts.insert(i, balances);
        }

        // Initialize the AMM liquidity pools with starting reserves for all token pairs.
        let mut pools = HashMap::new();
        for i in 0..tokens.len() {
            for j in (i + 1)..tokens.len() {
                let t_a = tokens[i];
                let t_b = tokens[j];
                pools.insert(
                    get_pool_key(t_a, t_b),
                    AmmPool::new(t_a, 1000.0, t_b, 1000.0),
                );
            }
        }

        Chain {
            state: ChainState {
                accounts,
                pools,
                blocks: Vec::new(),
                pending_transactions: Vec::new(),
                next_block_id: 0,
            },
        }
    }
}

/// Runs the API server to handle HTTP requests.
/// The server listens for transaction submissions and block queries.
async fn run_server(shared_state: Arc<Mutex<ChainState>>) {
    let app = Router::new()
        .route("/transaction", post(handle_transaction))
        .route("/blocks", get(get_blocks))
        .route("/rate", get(get_rate))
        .with_state(shared_state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    info!("listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

/// Defines the query parameters for the /blocks endpoint.
#[derive(Deserialize)]
struct GetBlocksQuery {
    /// The number of recent blocks to retrieve.
    n: Option<usize>,
}

/// Handles GET requests for retrieving recent blocks.
/// It returns the last n blocks, or all blocks if n is not specified.
async fn get_blocks(
    State(state): State<Arc<Mutex<ChainState>>>,
    Query(params): Query<GetBlocksQuery>,
) -> Json<Vec<Block>> {
    let state = state.lock().unwrap();
    let n = params.n.unwrap_or(state.blocks.len());
    let blocks_to_return = state.blocks.iter().rev().take(n).cloned().collect();
    Json(blocks_to_return)
}

/// Handles POST requests for submitting new transactions.
/// The received transaction is added to the pending transactions queue.
async fn handle_transaction(
    State(state): State<Arc<Mutex<ChainState>>>,
    Json(transaction): Json<Transaction>,
) {
    let mut state = state.lock().unwrap();
    info!("Received transaction: {:?}", transaction);
    // In a real application, more validation would be performed here.
    state.pending_transactions.push(transaction);
}

/// Computes a quoted output amount for a given input based on current pool reserves.
fn quote_amount_out(state: &ChainState, in_token: Token, out_token: Token, amount_in: f64) -> Option<f64> {
    if amount_in <= 0.0 {
        return None;
    }
    let pool_key = get_pool_key(in_token, out_token);
    if let Some(pool) = state.pools.get(&pool_key) {
        if let (Some(&in_reserve_val), Some(&out_reserve_val)) = (
            pool.reserves.get(&in_token),
            pool.reserves.get(&out_token),
        ) {
            if in_reserve_val > 0.0 && out_reserve_val > 0.0 {
                let amount_out = (out_reserve_val * amount_in) / (in_reserve_val + amount_in);
                if amount_out > 0.0 {
                    return Some(amount_out);
                }
            }
        }
    }
    None
}

#[derive(Deserialize)]
struct GetRateQuery {
    #[serde(rename = "in")] 
    in_token: String,
    #[serde(rename = "out")] 
    out_token: String,
    amount: Option<f64>,
}

#[derive(Serialize)]
struct GetRateResponse {
    amount_out: f64,
}

/// Handles GET requests for retrieving a quoted output for a token pair and amount.
async fn get_rate(
    State(state): State<Arc<Mutex<ChainState>>>,
    Query(params): Query<GetRateQuery>,
) -> Json<GetRateResponse> {
    let amount_in = params.amount.unwrap_or(1.0).max(0.0);
    let in_tok = Token::from_str(&params.in_token).expect("Invalid input token specified.");
    let out_tok = Token::from_str(&params.out_token).expect("Invalid output token specified.");

    let state = state.lock().unwrap();
    let quoted = quote_amount_out(&state, in_tok, out_tok, amount_in)
        .unwrap_or(0.0);
    Json(GetRateResponse { amount_out: quoted })
}

/// A loop that produces a new block every second.
/// It processes pending transactions and adds them to the new block.
async fn produce_blocks(state: Arc<Mutex<ChainState>>) {
    loop {
        sleep(Duration::from_secs(1)).await;
        let mut state = state.lock().unwrap();

        // Take all pending transactions to be processed in the current block.
        let transactions_to_process = std::mem::take(&mut state.pending_transactions);
        let mut successfully_processed_txs = Vec::new();

        // Each transaction is processed, and only successful ones are kept.
        for tx in transactions_to_process {
            let success = match tx.clone() {
                Transaction::Swap {
                    account,
                    in_token,
                    out_token,
                    amount,
                } => handle_swap(&mut state, account, in_token, out_token, amount),
                Transaction::Send {
                    from,
                    to,
                    token,
                    amount,
                } => handle_send(&mut state, from, to, token, amount),
            };
            if success {
                successfully_processed_txs.push(tx);
            }
        }

        // Create a new block with the successfully processed transactions.
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let new_block = Block {
            id: state.next_block_id,
            timestamp: current_time,
            transactions: successfully_processed_txs,
        };

        state.next_block_id += 1;
        info!("Producing block: {:?}", new_block);
        state.blocks.push(new_block);
    }
}

/// Handles the logic for a swap transaction.
/// It validates the swap and updates account balances and pool reserves.
fn handle_swap(
    state: &mut ChainState,
    account_id: AccountId,
    in_token: Token,
    out_token: Token,
    amount_in: f64,
) -> bool {
    // Check if the account exists and has sufficient balance.
    if let Some(account_balances) = state.accounts.get_mut(&account_id) {
        if let Some(in_token_balance) = account_balances.get_mut(&in_token) {
            if *in_token_balance >= amount_in {
                let pool_key = get_pool_key(in_token, out_token);
                // Check if the liquidity pool exists.
                if let Some(pool) = state.pools.get_mut(&pool_key) {
                    if let (Some(&in_reserve_val), Some(&out_reserve_val)) = (
                        pool.reserves.get(&in_token),
                        pool.reserves.get(&out_token),
                    ) {
                        if in_reserve_val > 0.0 && out_reserve_val > 0.0 {
                            // This calculation uses the constant product formula (x * y = k).
                            let amount_out = (out_reserve_val * amount_in)
                                / (in_reserve_val + amount_in);

                            // The swap is only valid if it results in a non-zero output.
                            if amount_out > 0.0 && out_reserve_val >= amount_out {
                                // Update the account's balances.
                                *in_token_balance -= amount_in;
                                let out_token_balance =
                                    account_balances.entry(out_token).or_insert(0.0);
                                *out_token_balance += amount_out;

                                // Update the pool's reserves.
                                let in_reserve = pool.reserves.get_mut(&in_token).unwrap();
                                *in_reserve += amount_in;
                                let out_reserve = pool.reserves.get_mut(&out_token).unwrap();
                                *out_reserve -= amount_out;
                                return true;
                            }
                        }
                    }
                }
            }
            else {
                tracing::error!("Transaction Failed: Account {:?} unable to swap {:?} {:?} for {:?}. Current balance: {:?}", account_id, amount_in, in_token, out_token, in_token_balance);
            }
        }
    }
    // Return false if any check fails.
    false
}

/// Handles the logic for a send transaction.
/// It validates the transaction and updates the balances of the sender and receiver.
fn handle_send(
    state: &mut ChainState,
    from: AccountId,
    to: AccountId,
    token: Token,
    amount: f64,
) -> bool {
    // Sending to oneself is not allowed.
    if from == to {
        return false;
    }

    // Both the sender and receiver must be existing accounts.
    if !state.accounts.contains_key(&from) || !state.accounts.contains_key(&to) {
        return false;
    }

    // The sender must have a sufficient balance of the specified token.
    if let Some(from_account) = state.accounts.get(&from) {
        if let Some(balance) = from_account.get(&token) {
            if *balance < amount {
                tracing::error!("Transaction Failed: Account {:?} unable to send {:?} {:?}. Current balance: {:?}", from, amount, token, balance);
                return false;
            }
        } else {
            // This case handles if the sender has no balance of the token at all.
            return false;
        }
    }

    // Debit the amount from the sender's account.
    if let Some(from_account) = state.accounts.get_mut(&from) {
        if let Some(balance) = from_account.get_mut(&token) {
            *balance -= amount;
        }
    }

    // Credit the amount to the receiver's account.
    if let Some(to_account) = state.accounts.get_mut(&to) {
        let balance = to_account.entry(token).or_insert(0.0);
        *balance += amount;
    }

    true
}
