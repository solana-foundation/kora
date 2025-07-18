pub fn create_transfer(from: &str, to: &str, amount: u64) -> String {
    format!("SignedTx: from={} to={} amount={}", from, to, amount)
}

