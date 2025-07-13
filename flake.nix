
{
  description = "Development environment with QEMU, virtualization, and build tools";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            # QEMU and virtualization
            qemu
            qemu-utils
            qemu_kvm
            bridge-utils
            libvirt
            virt-manager
            
            # Development tools
            curl
            gcc
            pkg-config
            openssl
            python3
            python3Packages.pip
            cmake
            ninja
            sudo
            git
            clang
          ];

          shellHook = ''
            echo "Development environment loaded!"
            echo "Available tools:"
            echo "  - QEMU/KVM virtualization stack"
            echo "  - Build tools (gcc, clang, cmake, ninja)"
            echo "  - Python 3 with pip"
            echo "  - libvirt for VM management"
            echo ""
            echo "Note: Some virtualization features may require additional system configuration"
          '';
        };
      }
    );
}
