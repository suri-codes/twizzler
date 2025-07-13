Vagrant.configure("2") do |config|
  # Use Ubuntu 22.04 base box
  config.vm.box = "ubuntu/jammy64"
  config.disksize.size = '150GB'
  # Configure VM resources
  config.vm.provider "virtualbox" do |vb|
    vb.memory = "8192"  # Increased from 4096 to 8192 MB
    vb.cpus = 8
    vb.customize ["modifyvm", :id, "--nested-hw-virt", "on"]
  end
  # Optional: Forward port if needed
  # config.vm.network "forwarded_port", guest: 80, host: 8080
  # Optional: Private network
  # config.vm.network "private_network", ip: "192.168.33.10"
  # Optional: Shared folder with permissions
  # config.vm.synced_folder ".", "/vagrant", type: "virtualbox", mount_options: ["dmode=775,fmode=664"]
  # Provisioning as root
  config.vm.provision "shell", inline: <<-SHELL
    export DEBIAN_FRONTEND=noninteractive
    apt-get update && apt-get install -y \
      qemu-system \
      qemu-utils \
      qemu-kvm \
      bridge-utils \
      cpu-checker \
      libvirt-daemon-system \
      libvirt-clients \
      virt-manager \
      curl \
      build-essential \
      pkg-config \
      libssl-dev \
      python3 \
      python3-pip \
      cmake \
      ninja-build \
      sudo \
      git \
      clang \
      && apt-get clean \
      && rm -rf /var/lib/apt/lists/*
    # Add vagrant user to kvm and libvirt groups
    usermod -a -G kvm,libvirt vagrant
    # Enable KVM status check
    kvm-ok || echo "⚠️  KVM not available inside guest"
  SHELL
  # Provisioning as vagrant user
  config.vm.provision "shell", privileged: false, inline: <<-SHELL
    set -e
    # Install Rust (if not already installed)
    if [ ! -x "$HOME/.cargo/bin/rustc" ]; then
      curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    fi
    # Add Rust to path permanently
    echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc
    echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.profile
    # Load Rust into this shell session
    export PATH="$HOME/.cargo/bin:$PATH"
    # Verify Rust
    rustc --version && cargo --version
    # Clone Twizzler if not already cloned
    if [ ! -d "$HOME/twizzler" ]; then
      git clone https://github.com/twizzler-operating-system/twizzler.git ~/twizzler
      cd ~/twizzler
      git submodule update --init --recursive
    fi
    # Build Twizzler
    cd ~/twizzler
    cargo bootstrap
    cargo build-all
    echo "------------------------------------------------------------"
    echo "🛠️  Twizzler OS environment ready!"
    echo "📂  Source:     ~/twizzler"
    echo "🚀  To build:   cd ~/twizzler && cargo build-all"
    echo "------------------------------------------------------------"
  SHELL
  # Run on every VM boot
  config.vm.provision "shell", run: "always", privileged: false, inline: <<-SHELL
    export PATH="$HOME/.cargo/bin:$PATH"
    eval `(ssh-agent)`
    ssh-add ~/.ssh/github
  SHELL
end
