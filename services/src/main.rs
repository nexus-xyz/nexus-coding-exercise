use clap::{Parser};
use services::chain;
use std::str::FromStr;

/// Defines the command-line arguments for the application.
/// This structure uses `clap` to parse and validate arguments.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Flag to run in client mode.
    #[arg(long)]
    client: bool,

    /// Flag to run in server mode.
    #[arg(long)]
    server: bool,

    /// Flag to start the blockchain simulation.
    #[arg(long)]
    start_chain: bool,

    /// The number of accounts to create when starting the chain.
    #[arg(long, default_value_t = 10)]
    num_accounts: u32,

    /// The arguments for a swap transaction.
    /// Expects account ID, input token, output token, and amount.
    #[arg(long, num_args = 4, value_names = ["ACCOUNT_ID", "IN_TOKEN", "OUT_TOKEN", "AMOUNT"])]
    swap: Option<Vec<String>>,

    /// The arguments for a send transaction.
    /// Expects sender ID, receiver ID, amount, and token.
    #[arg(long, num_args = 4, value_names = ["SENDER_ID", "RECEIVER_ID", "AMOUNT", "TOKEN"])]
    send: Option<Vec<String>>,

    /// Fetches and prints the most recent n blocks.
    /// Defaults to 10 if no number is provided.
    #[arg(long, num_args = 0..=1, default_missing_value = "10")]
    block: Option<String>,
}

/// The entry point of the application.
/// It parses command-line arguments and executes the corresponding logic.
#[tokio::main]
async fn main() -> std::io::Result<()> {
    let args = Args::parse();

    // The main logic flow is determined by which command-line argument is provided.
    // The application handles one primary command at a time.
    if let Some(swap_args) = args.swap {
        // This block handles the --swap command.
        // It parses the arguments, creates a swap transaction, and sends it to the chain.
        let account: u32 = swap_args[0]
            .parse()
            .expect("Invalid account ID. Must be a number.");
        let in_token =
            chain::Token::from_str(&swap_args[1]).expect("Invalid input token specified.");
        let out_token =
            chain::Token::from_str(&swap_args[2]).expect("Invalid output token specified.");
        let amount: f64 = swap_args[3]
            .parse()
            .expect("Invalid amount specified. Must be a number.");

        let tx = chain::Transaction::Swap {
            account,
            in_token,
            out_token,
            amount,
        };

        let client = reqwest::Client::new();
        let res = client
            .post("http://127.0.0.1:3000/transaction")
            .json(&tx)
            .send()
            .await
            .expect("Failed to send transaction to the chain.");

        if res.status().is_success() {
            println!("Swap transaction sent to chain successfully.");
        } else {
            println!(
                "Failed to send swap transaction. Status: {}",
                res.status()
            );
        }
    } else if let Some(send_args) = args.send {
        // This block handles the --send command.
        // It parses the arguments, creates a send transaction, and sends it to the chain.
        let sender: u32 = send_args[0]
            .parse()
            .expect("Invalid sender ID. Must be a number.");
        let receiver: u32 = send_args[1]
            .parse()
            .expect("Invalid receiver ID. Must be a number.");
        let amount: f64 = send_args[2]
            .parse()
            .expect("Invalid amount. Must be a number.");
        let token = chain::Token::from_str(&send_args[3]).expect("Invalid token specified.");

        let tx = chain::Transaction::Send {
            from: sender,
            to: receiver,
            amount,
            token,
        };

        let client = reqwest::Client::new();
        let res = client
            .post("http://127.0.0.1:3000/transaction")
            .json(&tx)
            .send()
            .await
            .expect("Failed to send transaction to the chain.");

        if res.status().is_success() {
            println!("Send transaction sent to chain successfully.");
        } else {
            println!(
                "Failed to send send transaction. Status: {}",
                res.status()
            );
        }
    } else if let Some(n_str) = args.block {
        // This block handles the --block command.
        // It fetches the n most recent blocks from the chain and prints them.
        let n: usize = n_str.parse().expect("Invalid number of blocks.");
        let client = reqwest::Client::new();
        let res = client
            .get(format!("http://127.0.0.1:3000/blocks?n={}", n))
            .send()
            .await
            .expect("Failed to get blocks from the chain.");
        if res.status().is_success() {
            let blocks: Vec<chain::Block> = res
                .json()
                .await
                .expect("Failed to parse blocks from response.");
            println!("Most recent {} blocks:", blocks.len());
            println!("{:#?}", blocks);
        } else {
            println!("Failed to get blocks. Status: {}", res.status());
        }
    } else if args.start_chain {
        // This block handles the --start-chain command.
        // It initializes and starts the blockchain simulation.
        chain::start_chain(args.num_accounts).await;
    } else {
        // If no command is provided, print a usage message.
        println!("Please specify a command, e.g., --start-chain, --send, --swap, or --block.");
    }
    Ok(())
}
