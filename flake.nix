{
    description = "tgstation-server-info-fetcher";

    inputs = {};

    outputs = { nixpkgs, ... }: {
        nixosModules = {
            default = { ... }: {
                imports = [ ./service.nix ];
            };
        };
    };
}
