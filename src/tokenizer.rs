use anyhow::{Context, Result};

pub trait Tokenizer: Send + Sync {
    fn count_tokens(&self, text: &str) -> usize;
}

#[cfg(feature = "token-counting")]
pub struct O200kTokenizer {
    bpe: tiktoken_rs::CoreBPE,
}

#[cfg(feature = "token-counting")]
impl O200kTokenizer {
    pub fn new() -> Result<Self> {
        let bpe = tiktoken_rs::o200k_base().context("Failed to initialize o200k_base tokenizer")?;
        Ok(Self { bpe })
    }
}

#[cfg(feature = "token-counting")]
impl Tokenizer for O200kTokenizer {
    fn count_tokens(&self, text: &str) -> usize {
        self.bpe.encode_with_special_tokens(text).len()
    }
}

#[cfg(not(feature = "token-counting"))]
pub struct DummyTokenizer;

#[cfg(not(feature = "token-counting"))]
impl Tokenizer for DummyTokenizer {
    fn count_tokens(&self, _text: &str) -> usize {
        0
    }
}

#[cfg(not(feature = "token-counting"))]
impl DummyTokenizer {
    pub fn new() -> anyhow::Result<Self> { Ok(Self) }
}