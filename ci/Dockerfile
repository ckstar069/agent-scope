# AgentScope CI 基础镜像
# 预置：Node.js 20、Rust 1.95.0、Tauri Linux 依赖、Playwright Chromium、常用 cargo 工具

FROM ubuntu:22.04

ARG NODE_VERSION=20
ARG RUST_VERSION=1.95.0
ARG CARGO_BINSTALL_VERSION=1.19.1

ENV DEBIAN_FRONTEND=noninteractive
ENV RUSTUP_HOME=/usr/local/rustup
ENV CARGO_HOME=/usr/local/cargo
ENV PATH=/usr/local/cargo/bin:$PATH
ENV NODE_VERSION=${NODE_VERSION}

# 1. 安装系统依赖（Tauri + 构建工具）
RUN apt-get update -qq && \
    apt-get install -y -qq --no-install-recommends \
        libwebkit2gtk-4.1-dev \
        libayatana-appindicator3-dev \
        librsvg2-dev \
        patchelf \
        curl \
        build-essential \
        git \
        ca-certificates \
        pkg-config \
        python3 \
        python3-distutils && \
    rm -rf /var/lib/apt/lists/*

# 2. 安装 Node.js（NodeSource）
RUN curl -fsSL https://deb.nodesource.com/setup_${NODE_VERSION}.x | bash - && \
    apt-get install -y -qq --no-install-recommends nodejs && \
    rm -rf /var/lib/apt/lists/*

# 3. 安装 Rust（固定版本）
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | \
    sh -s -- -y --default-toolchain ${RUST_VERSION} --no-modify-path && \
    chmod -R a+w $RUSTUP_HOME $CARGO_HOME

# 4. 安装 cargo-binstall
RUN curl -L --proto '=https' --tlsv1.2 -sSf \
    https://github.com/cargo-bins/cargo-binstall/releases/download/v${CARGO_BINSTALL_VERSION}/cargo-binstall-x86_64-unknown-linux-gnu.tgz | \
    tar -xz -C /usr/local/cargo/bin/

# 5. 安装 cargo-audit
RUN cargo binstall cargo-audit --locked --no-confirm

# 6. 安装 Playwright Chromium 及系统依赖
# 先装一个占位目录让 Playwright 安装器能找到位置
RUN mkdir -p /ci/.cache/ms-playwright
ENV PLAYWRIGHT_BROWSERS_PATH=/ci/.cache/ms-playwright

# 下载 Playwright 安装器但只安装 chromium 依赖（不下载浏览器，因为浏览器会缓存在项目目录）
RUN npm install -g @playwright/test && \
    npx playwright install-deps chromium

# 7. 验证安装
RUN node --version && \
    rustc --version && \
    cargo --version && \
    cargo fmt --version && \
    cargo clippy --version && \
    which cargo-audit && \
    which cargo-binstall

# 8. 设置工作目录
WORKDIR /builds

# 镜像元数据
LABEL maintainer="agent-scope-team" \
      description="AgentScope CI base image with Node.js 20, Rust 1.95.0, Tauri deps, Playwright deps"
