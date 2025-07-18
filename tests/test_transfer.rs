#[cfg(test)]
mod tests {
    use super::super::lib::transfer::create_transfer;

    #[test]
    fn test_transfer_logic() {
        let tx = create_transfer("alice", "bob", 500);
        assert_eq!(tx, "SignedTx: from=alice to=bob amount=500");
    }
}

