FROM ubuntu:22.04

# Avoid interactive prompts during package installation
ENV DEBIAN_FRONTEND=noninteractive

# Update package list and install QEMU, Rust dependencies, and build tools
RUN apt-get update && apt-get install -y \
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

# Install Rust via rustup as root
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Verify Rust installation
RUN rustc --version && cargo --version

# Set working directory
WORKDIR /root

RUN git clone https://github.com/twizzler-operating-system/twizzler.git
WORKDIR /root/twizzler
RUN git submodule update --init --recursive
RUN cargo bootstrap
RUN cargo build-all 

# Default command
CMD ["/bin/bash"]
