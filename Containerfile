FROM fedora:40

# 1. Install system dependencies (Compiled once)
RUN dnf install -y \
    gcc pkgconf-pkg-config \
    wayland-devel cairo-devel cairo-gobject-devel librsvg2-devel libxkbcommon-devel \
    glib2-devel pango-devel \
    rpm-build tar flatpak-builder flatpak \
    && dnf clean all

# 2. Install latest Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# 3. Pre-fetch Flatpak Runtimes
RUN flatpak remote-add --if-not-exists flathub https://flathub.org/repo/flathub.flatpakrepo \
    && flatpak install -y flathub org.freedesktop.Sdk//23.08 org.freedesktop.Platform//23.08 \
    && flatpak install -y flathub org.freedesktop.Sdk.Extension.rust-stable//23.08

WORKDIR /build
# No COPY . . here! The code will be mounted at runtime.

CMD ["./package.sh"]
