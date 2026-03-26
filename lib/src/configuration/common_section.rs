use std::net::IpAddr;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct CommonSection
{
  pub listening_address: IpAddr,
  pub target_address: IpAddr,
}
