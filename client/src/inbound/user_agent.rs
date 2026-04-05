use rand::{Rng, RngExt, seq::IndexedRandom};

#[derive(Debug, Clone, Copy)]
pub enum Browser {
  Chrome,
  Firefox,
  Edge,
  Safari,
  Opera,
  YandexBrowser,
}

#[derive(Debug, Clone, Copy)]
pub enum Os {
  Windows,
  MacintoshIntel,
  Linux,
  Ubuntu,
}

pub struct UserAgent {
  pub value: String,
  pub os: Os,
  pub browser: Browser,
}

fn random_version(rng: &mut impl Rng, major_min: u32, major_max: u32) -> String {
  format!("{}.0.0.0", 
    rng.random_range(major_min..=major_max),
  )
}

pub fn get_random_user_agent() -> UserAgent
{
  let mut rng = rand::rng();

  let browser = *[
    Browser::Chrome, Browser::Firefox, Browser::Edge, 
    Browser::Safari, Browser::Opera, Browser::YandexBrowser
  ].choose(&mut rng).unwrap();

  let os = match browser {
    Browser::Safari => Os::MacintoshIntel,
    _ => *[Os::Windows, Os::MacintoshIntel, Os::Linux].choose(&mut rng).unwrap(),
  };

  let chrome_v = random_version(&mut rng, 135, 146);
  let nt_v = if rng.random_bool(0.65) { "10.0" } else { "11.0" };
  let mac_v = if rng.random_bool(0.43) { "10_15_7" } else { "11_0_1" };

  let platform = match os {
    Os::Windows => format!("Windows NT {}; Win64; x64", nt_v),
    Os::MacintoshIntel => format!("Macintosh; Intel Mac OS X {}", mac_v),
    Os::Linux => "X11; Linux x86_64".to_string(),
    Os::Ubuntu => "X11; Ubuntu; Linux x86_64".to_string(),
  };

  let value = match browser {
    Browser::Firefox => {
      let ff_v = rng.random_range(135..=149);
      format!("Mozilla/5.0 ({}; rv:{}.0) Gecko/20100101 Firefox/{}.0", platform, ff_v, ff_v)
    }
    Browser::Safari => {
      let saf_v = rng.random_range(15..=17);
      format!("Mozilla/5.0 ({}) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/{}.0 Safari/605.1.15", platform, saf_v)
    }
    _ => {
      let base = format!("Mozilla/5.0 ({}) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/{} Safari/537.36", platform, chrome_v);
      match browser {
          Browser::Edge => format!("{} Edg/{}", base, chrome_v),
          Browser::Opera => format!("{} OPR/{}", base, random_version(&mut rng, 110, 112)),
          Browser::YandexBrowser => format!("{} YaBrowser/{} Yowser/2.5", base, random_version(&mut rng, 23, 24)),
          _ => base,
      }
    }
  };
  
  UserAgent { value, os, browser }
}