use rayon::prelude::*;
use sha2::{Digest as _, Sha256};
use anyhow::{Result,anyhow};
use tokio::task::{JoinHandle, spawn_blocking};

/// Задача для поиска хеша
#[derive(Clone)]
pub struct PowChallenge {
  pub input: String,
  pub difficulty: usize
}

/// Решает PoW задачу, перебирая nonce до тех пор, пока не будет найден хэш,
/// начинающийся с нужного количества нулей
fn solve_pow(challenge: PowChallenge) -> Result<String>
{
  let full_bytes = challenge.difficulty / 2;
  let has_half_byte = challenge.difficulty % 2 != 0;
  (0..u64::MAX)
    .into_par_iter()
    .find_first(|&nonce| {
      let mut hasher = Sha256::new();
      let data = format!("{}{}", challenge.input, nonce);

      hasher.update(data.as_bytes());
      let hash = hasher.finalize();

      for i in 0..full_bytes {
        if hash[i] != 0 {
          return false;
        }
      }

      if has_half_byte {
        if (hash[full_bytes] & 0xF0) != 0 {
          return false;
        }
      }

      true
    })
    .map(|nonce| {
      let data = format!("{}{}", challenge.input, nonce);
      let mut hasher = Sha256::new();
      hasher.update(data.as_bytes());
      hex::encode(hasher.finalize())
    })
    .ok_or_else(|| anyhow!("Failed to solve PoW challenge"))
}

/// Асинхронная обёртка для решения задачи от ВК
pub fn solve_pow_async(challenge: PowChallenge) -> JoinHandle<Result<String>>
{
  spawn_blocking(move || solve_pow(challenge))
}