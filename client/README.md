# TURN Proxy (Клиент)

Слушает порт (например, 51820), на который стучится, например, WireGuard. Приложение этот трафик оборачивает в DTLS и
пересылает его на TURN, указав ему адресата, которым является Ваш серевер. TURN сервер видит лишь IP и порт вашего
сервера и легитимный DTLS трафик, внутри которого, скорее всего, медиа-данные и пересылает DTLS трафик на конечный
сервер.

В коде восходящее соединение буквально является такой обёрткой: `[ DTLS Connection [ Targeted Connection [
TURN Connection [ UDP Connection ] ] ] ]`, проще говоря, матрёшка. Нисходящее соединение то же самое, только наоборот.

### Windows

Для ОС Windows можно скачать во вкладке ["релизы"](https://github.com/Urtyom-Alyanov/turn-proxy/releases/latest).

### Linux-based

На данный момент доступны [`flake.nix`](./flake.nix) для пакетного менеджера Nix вместе с модулем для NixOS, а также
[`PKGBUILD`](./PKGBUILD) для Arch Linux

#### Быстрая установка (Debian/Ubuntu/Fedora/и производные)

```bash
curl -sSL https://raw.githubusercontent.com/Urtyom-Alyanov/turn-proxy-server/master/install-server.sh | bash
```

##### Минимальные требования

<!-- - Ubuntu 21.10 (или 22.04 LTS) -->
<!-- - Debian 13 (Bookworm) -->
<!-- - Fedora 35
- RHEL / CentOS 9 -->

Проще говоря, нужна минимальная версия glibc: `2.31`

#### NixOS

Для наилучшей операционной системы модно, можно и надо использовать модули, а если быть точнее, то модуль который
можно импортировать с помощью [`flake.nix`](../flake.nix). Также его декларация находится здесь: [`nix/module.nix`](./nix/module.nix)

В вашем flake.nix просто укажите:

```nix
{
  inputs = {
    turn-proxy.url = "github:Urtyom-Alyanov/turn-proxy";
  };
  outputs = { turn-proxy }: {
    # импортируйте куда нибудь
    # модуль turn-proxy.nixosModules.client
    # или же пакет turn-proxy.packages.${pkgs.system}.client
  };
}
```

#### Для Arch Linux (PKGBUILD)

```shell
# Когда опубликуется на AUR
#git clone [https://aur.archlinux.org/turn-proxy-client-rs.git](https://aur.archlinux.org/turn-proxy-client-rs.git)
#cd turn-proxy-client-rs
#makepkg -si

# Поэтому пока так
git clone https://github.com/Urtyom-Alyanov/turn-proxy.git
cd turn-proxy/client
makepkg -si
```

#### Dockerfile

Также есть [Dockerfile](./Dockerfile), но сам пакет не опубликован на Dockerhub, делайте с ним что хотите.

## Использование

По умолчанию программа ищет конфигурацию в `/etc/turn-proxy/server/config.toml`, однако можно задать и иной путь
с помощью `--config {путь}`.

### Конфигурация

Конфигурационный файл имеет следующую структуру:

```toml
[common]
listening_on = "127.0.0.1:51821" # Адрес и порт 
peer_addr = "127.0.0.1:56040" # Адрес и порт удалённого сервера
interface_addr = "10.77.173.50" # Необязательнеый параметр, который задаёт статически адрес интерфейса (то есть адрес устройства в сети)

[[providers]]
priority = 1 # Приоритет проксированния
using_udp = true # НЕ РЕАЛИЗОВАНО, ПО УМОЛЧАНИЮ РАБОТАЕТ ТОЛЬКО ЧЕРЕЗ UDP
using_dtls_obfuscation = true # НЕ РЕКОМЕНДУЕТСЯ ОТКЛЮЧАТЬ. Использовать DTLS в качестве транспорта
threads = 16 # Потоки, то есть пользователи-гости, сидящие в видео-конференции

[providers.details]
provider = "default"
kind = "vk_calls" # тип прокси, также доступен "yandex_telemost"
link = "https://vk.com/call/join/..." # Ссылка на видеоконференцию
```

Если вы используете NixOS, то можно использовать модуль, пример ниже

```nix
{
  services.turn-proxy.server = {
    enable = true; # Включаем шарманку
    
  };
}
```

Также можно указать **в качестве аргументов (не поддерживает приоритеты)**, но высё же рекомендуется использовать конфигурацию, если вы собираетесь запускать обфускатор вручную:

```shell
turn-proxy-client --no-config --listening-on=0.0.0.0:56000 --peer_addr=127.0.0.1:51820 --threads=16 default --kind=vk_calls --link=https://vk.com/call/join/...
```

### Сервисы

Однако рекомендуется запускать его с помощью сервиса, для systemd сервис называется `turn-proxy-client.service`,
манипуляции с ним такие же, как и с другими сервисами, учтите, что последние 2 команды не применимы к NixOS из-за её
природы

```shell
systemctl start turn-proxy-client.service # Чтобы его запустить
systemctl stop turn-proxy-client.service # Чтобы его остановить
systemctl restart turn-proxy-client.service # Чтобы его перезапустить (если он залагал или конфигурацию поменяли)
systemctl enable --now turn-proxy-client.service # Чтобы его запускать вместе с системой (флаг --now его запустит в ту же секунду)
systemctl disable --now turn-proxy-client.service # Чтобы его НЕ запускать вместе с системой (флаг --now его остановит в ту же секунду)
```

В качестве адресата можно использовать любые протоколы, работающие поверх UDP, например WireGuard или Hysteria2.

**ПРИ ИСПОЛЬЗОВАНИИ WIREGUARD рекомендуется поставить MTU равным 1280-1380, так как TURN и DTLS добавляют свои заголовки,
что может привести к фрагментации пакетов, так как пакеты могут уже не влезать в стандартное ограничение в 1500 байт,
что приведёт к резкому снижению скорости, а она и так невелика.**

---

_Сервер лицензирован под лицензией [AGPLv3](./LICENSE)_
