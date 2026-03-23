use serde::Deserialize;

#[derive(clap::ValueEnum, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "snake_case")]
pub enum DefaultProvider
{
  #[default]
  VkCalls,
  YandexTelemost,
}

#[derive(Deserialize, Debug, Default, Clone)]
#[serde(tag = "provider", rename_all = "snake_case")]
pub enum ProviderDetails
{
  #[default]
  Direct,
  Default
  {
    kind: DefaultProvider, link: String
  },
  Custom
  {
    username: String,
    password: String,
    turn_address: String,
    stun_address: String,
    realm: String,
  },
}

#[derive(Deserialize, Debug, Default)]
pub struct CommonConfiguration
{
  /// Адрес входа/выхода
  pub listening_on: String,
  /// Конечный сервер
  pub peer_addr: String,
  /// Выписывание адресов
  pub write_addr: Option<bool>,
}

#[derive(Deserialize, Debug, Default, Clone)]
pub struct ProviderConfiguration
{
  /// Приоритет, если не задан то Direct -> Custom -> VK Calls ->
  /// Yandex.Telemost
  pub priority: Option<u32>,
  /// Не использовать UDP для TURN сервера поставщика (может понизить скорость),
  /// не знаю зачем это кому-то, на другие параметры не влияет
  ///
  /// По умолчанию `true`
  pub using_udp: bool,
  /// Не использовать DTLS обфускацию для поставщика (может увеличить скорость,
  /// но также может увеличить шанс на блокировку)
  ///
  /// По умолчанию `true`.
  ///
  /// НЕ РЕКОМЕНДУЕТСЯ ОТКЛЮЧАТЬ
  pub using_dtls_obfuscation: bool,
  /// Специфичные поля для разных поставщиков TURN серверов (в том числе и
  /// Direct)
  pub details: ProviderDetails,
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
  pub threads: Option<usize>,
}

#[derive(Deserialize, Debug, Default)]
pub struct AppConfiguration
{
  #[serde(default)]
  pub common: CommonConfiguration,
  #[serde(default)]
  pub providers: Vec<ProviderConfiguration>,
}
