let
  pkgs =
    import
      #(fetchTarball "https://github.com/NixOS/nixpkgs/archive/cf8cc1201be8bc71b7cbbbdaf349b22f4f99c7ae.tar.gz")
      (fetchTarball "https://github.com/NixOS/nixpkgs/archive/refs/tags/25.05-pre.tar.gz")
      { };
in
pkgs.buildFHSEnv {
  name = "come_on";

  #packages = [
  #  (pkgs.python312.withPackages (
  #    python-pkgs: with python-pkgs; [
  #      numpy
  #      pandas
  #      requests
  #      #websocket
  #      #tk
  #      #temp
  #      #alpaca-trade-api
  #      pillow
  #      plotly
  #      ipython
  #      kaleido
  #      pio
  #    ]
  #  ))
  #];
  runScript = ''
      source .env/bin/activate.fish
  '';
}
