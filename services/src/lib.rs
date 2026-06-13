pub mod chain;


#[cfg(test)]
mod tests {
    use super::chain;
    use reqwest::Client;
    use serde::Deserialize;
    use std::time::Duration;
    use tokio::time::sleep;

    #[derive(Deserialize)]
    struct RateResp {
        amount_out: f64,
    }

    #[tokio::test]
    async fn start_chain_swap_and_rate_changes() {
        // Start the chain server in background
        tokio::spawn(async move {
            chain::start_chain(2).await;
        });

        // Give the server a moment to start
        sleep(Duration::from_millis(400)).await;

        let client = Client::new();

        // Get initial quote for swapping NEX -> ETH with amount 10
        let before: RateResp = client
            .get("http://127.0.0.1:3000/rate?in=NEX&out=ETH&amount=10")
            .send()
            .await
            .expect("rate request failed")
            .json()
            .await
            .expect("invalid rate response");

        // Submit a swap transaction for account 0: NEX -> ETH amount 10
        let tx = chain::Transaction::Swap {
            account: 0,
            in_token: chain::Token::NEX,
            out_token: chain::Token::ETH,
            amount_in: 100.0,
            amount_out: 0.0,
        };

        let _ = client
            .post("http://127.0.0.1:3000/transaction")
            .json(&tx)
            .send()
            .await
            .expect("failed to post transaction");

        // Wait for the block producer to include the transaction
        sleep(Duration::from_millis(1300)).await;

        // Get quote again after swap
        let after: RateResp = client
            .get("http://127.0.0.1:3000/rate?in=NEX&out=ETH&amount=10")
            .send()
            .await
            .expect("rate request failed")
            .json()
            .await
            .expect("invalid rate response");

        // After consuming ETH liquidity with a NEX->ETH swap, the quoted output should decrease
        assert!(after.amount_out < before.amount_out, "expected quote to decrease after swap: before={} after={}", before.amount_out, after.amount_out);
    }
}
