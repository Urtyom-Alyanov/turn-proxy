self: { config, lib, ... }:
with lib;
let
  opt = config.services.turn-proxy.client;
  tomlFormat = pkgs.formats.toml { };

  toSnake = name:
    let
      chars = stringToCharacters name;
      process = char: if char == toUpper char && char != toLower char
                      then "_" + toLower char
                      else char;
    in concatStrings (map process chars);

  mapKeysToSnake = value:
    if isAttrs value then
      mapAttrs' (n: v: nameValuePair (toSnake n) (mapKeysToSnake v)) value
    else if isList value then
      map mapKeysToSnake value
    else
      value;

  providerModule = { name, ... }: {
    options = {
      priority = mkOption {
        type = types.nullOr types.ints.unsigned;
        default = null;
        description = "Приоритет провайдера.";
      };
      usingUdp = mkOption {
        type = types.bool;
        default = true;
        description = "Использовать UDP для TURN сервера.";
      };
      usingDtlsObfuscation = mkOption {
        type = types.bool;
        default = true;
        description = "Использовать DTLS обфускацию (рекомендуется true).";
      };
      threads = mkOption {
        type = types.nullOr types.ints.positive;
        default = null;
        description = "Количество потоков (участников конференции).";
      };
      details = {
        provider = mkOption {
          type = types.enum [ "direct" "default" "custom" ];
          default = "direct";
          description = "Тип провайдера (Direct, Default или Custom).";
        };

        # Default
        kind = mkOption {
          type = types.nullOr (types.enum [ "vk_calls" "yandex_telemost" ]);
          default = null;
          description = "Для типа 'default': выбор конкретного сервиса.";
        };
        link = mkOption {
          type = types.nullOr types.str;
          default = null;
          description = "Ссылка для подключения (для 'default').";
        };

        # Custom
        username = mkOption { type = types.nullOr types.str; default = null; };
        password = mkOption { type = types.nullOr types.str; default = null; };
        turnAddress = mkOption { type = types.nullOr types.str; default = null; };
        stunAddress = mkOption { type = types.nullOr types.str; default = null; };
        realm = mkOption { type = types.nullOr types.str; default = null; };
      };
    };
  };

  appConfigRaw = {
    common = filterAttrs (n: v: v != null) cfg.common;
    providers = map (p:
      let
        base = filterAttrs (n: v: v != null) {
          inherit (p) priority usingUdp usingDtlsObfuscation threads;
        };
        details = if p.details.provider == "direct" then { provider = "direct"; }
                  else if p.details.provider == "default" then {
                    provider = "default";
                    inherit (p.details) kind link;
                  }
                  else {
                    provider = "custom";
                    inherit (p.details) username password;
                    turn_address = p.details.turnAddress;
                  };
      in base // { inherit details; }
    ) cfg.providers;
  };

  finalConfig = mapKeysToSnake appConfigRaw;
in
{
  options.services.turn-proxy.client = {
    enable = mkEnableOption "DTLS Turn Proxy Client";

    package = mkOption {
      type = types.package;
      default = self.packages.${pkgs.system}.client;
    };

    common = {
      listeningOn = mkOption {
        type = types.str;
        default = "127.0.0.1:51820";
        description = "Адрес входа/выхода";
      };
      peerAddr = mkOption {
        type = types.str;
        description = "Конечный сервер";
      };
      interfaceAddr = mkOption {
        type = types.nullOr types.str;
        default = null;
        description = "Адрес интерфейса сети";
      };
    };

    providers = mkOption {
      type = types.listOf (types.submodule providerModule);
      default = [ ];
      description = "Список конфигураций провайдеров";
    };
  };

  config = mkIf opt.enable {
    environment.etc."turn-proxy/client/config.toml".source =
      tomlFormat.generate "turn-proxy-client-config" finalConfig;

    systemd.services.turn-proxy-client = {
      description = "DTLS TURN Proxy Client";
      after = [ "network.target" ];
      wantedBy = [ "multi-user.target" ];
      serviceConfig = {
        ExecStart = "${opt.package}/bin/turn-proxy-client --config /etc/turn-proxy/client/config.toml from-config-file";
        Restart = "always";
        DynamicUser = true;
        ConfigurationDirectory = "turn-proxy-client";
      };
    };
  };
}
