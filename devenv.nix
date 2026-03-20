{ pkgs, lib, config, inputs, ... }:

{
  packages = [
    pkgs.openssl
  ];

  languages.rust = {
    enable = true;
    channel = "stable";
  };
}
