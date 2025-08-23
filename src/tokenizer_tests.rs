#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dummy_counts_zero() {
        #[cfg(not(feature = "token-counting"))]
        {
            let t = DummyTokenizer::new().unwrap();
            assert_eq!(t.count_tokens("hello"), 0);
        }
    }
}