use clap::{Args as ClapArgs, Parser, Subcommand};

use crate::configuration::configuration::DefaultProvider;

#[cfg(target_os = "windows")]
const DEFAULT_CONFIG_PATH: &str = ".\\config.toml";

#[cfg(target_os = "linux")]
const DEFAULT_CONFIG_PATH: &str = "/etc/turn-proxy/client/config.toml";

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args
{
  #[arg(long, short, default_value = DEFAULT_CONFIG_PATH, help = "Путь к конфигурации")]
  pub config: String,

  #[arg(long, short = 'L', help = "Слушать выходной адрес")]
  pub listening_on: Option<String>,

  #[arg(long, short = 'P', help = "Адрес назначения")]
  pub peer_addr: Option<String>,

  #[arg(long, short, help = "Вывод адресов")]
  pub write_addr: Option<bool>,

  #[command(flatten)]
  pub provider_common: Option<ProviderCliArgs>,

  #[command(subcommand)]
  pub provider_type: Option<ProviderType>,
}

#[derive(ClapArgs, Debug, Default)]
pub struct ProviderCliArgs
{
  /// Количество потоков, выглядит как количество участников в конференции,
  /// большие значения могут вызвать подозрения, так как с одного IP адреса
  /// идёт подключается одновременно к одному звонку условно 16 человек, что,
  /// довольно, странно.
  ///
  /// Если поставщик direct, то поле игнорируется
  ///
  /// Не рекомендуется указывать большие значения, однако может существенно
  /// увеличить скорость, если со стороны поставщика имеется ограничение по
  /// скорости для участника конференции.
  #[arg(
    long,
    short = 'n',
    help = "Количество потоков (участников в звонке), игнорируется при direct"
  )]
  pub threads: Option<u32>,

  #[arg(long, short = 'U', default_value_t = true, help = "Использовать UDP")]
  pub using_udp: bool,

  #[arg(
    long,
    short = 'D',
    default_value_t = true,
    help = "Использовать DTLS обфускацию, не рекомендуется отключать \
    при использовании TURN-сервера"
  )]
  pub using_dtls_obfuscation: bool,
}

#[derive(Subcommand, Debug)]
pub enum ProviderType
{
  Direct,
  Default
  {
    #[arg(long = "provider", short = 'p', help = "Выбранный провайдер")]
    kind: DefaultProvider, // Доступные провайдеры
    #[arg(long, short = 'l', help = "Ссылка на звонок/конференцию")]
    link: String,
  },
  Custom
  {
    #[arg(long, short = 'u', help = "Имя пользователя для TURN")]
    username: String,
    #[arg(long, short = 'p', help = "Пароль для TURN")]
    password: String,
    #[arg(long, short = 't', help = "Адрес TURN-сервера")]
    turn_address: String,
    #[arg(long, short = 's', help = "Адрес STUN-сервера")]
    stun_address: String,
    #[arg(long, short = 'r', help = "Realm конференции")]
    realm: String,
  },
  FromConfigFile,
}
