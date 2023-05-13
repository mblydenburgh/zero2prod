#[derive(Debug)]
pub struct IdempotencyKey(String);

impl TryFrom<String> for IdempotencyKey {
    type Error = anyhow::Error;

    fn try_from(s: String) -> Result<Self, Self::Error> {
       if s.is_empty() {
           anyhow::bail!("Idempotency Key cannot be empty")
       }

       let max_length = 50;
       if s.len() >= max_length {
           anyhow::bail!("Idempotency Key must be shorter than {}", max_length)
       }
       Ok(Self(s))
    }
}

impl From<IdempotencyKey> for String {
    fn from(k: IdempotencyKey) -> Self {
       k.0 
    }
}

impl AsRef<str> for IdempotencyKey {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
