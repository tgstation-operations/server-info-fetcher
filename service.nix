inputs@{
  config,
  pkgs,
  lib,
  ...
}:

let
  package-name = "tgstation-server-info-fetcher";
  package = pkgs.rustPlatform.buildRustPackage {
    pname = package-name;
    version = "0.1.0";
    src = ./.;
    cargoHash = "sha256-x1Ui63dVxEKULKBsynmMv0cIK/ZzkfhRTOQArmUOuP4=";
  };
  cfg = config.services.tgstation-server-info-fetcher;
  servers-command-option = lib.concatStringsSep "," cfg.servers;
in
{
  options.services.tgstation-server-info-fetcher = {
    enable = lib.mkEnableOption "tgstation-server-info-fetcher";
    groupname = lib.mkOption {
      type = lib.types.nonEmptyStr;
      default = package-name;
      description = ''
        The name of group the user used to run ${package-name} will belong to.
      '';
    };
    servers = lib.mkOption {
      type = lib.types.listOf lib.types.nonEmptyStr;
      default = [];
      description = ''
        Internal BYOND server addresses with ports
      '';
      example = [ "server1.example.com:1337" "server2.example.com:1234" ];
    };
  };

  config = lib.mkIf cfg.enable {
    # Server Info Fetcher
    systemd.services."${package-name}" = {
      wantedBy = ["multi-user.target"];
      serviceConfig = {
        Restart = "always";
        RestartSec = "5s";
        RestartMaxDelaySec = "5s";
        DynamicUser = true;
        Group = cfg.groupname;
        RuntimeDirectory = package-name;
        ExecStart = "${package}/bin/server-info-fetcher --failure-tolerance all --servers ${servers-command-option} /run/${package-name}/serverinfo.json";
      };
    };
  };
}
