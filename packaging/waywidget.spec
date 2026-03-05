Name:           waywidget
Version:        0.1.3
Release:        1%{?dist}
Summary:        SVG-to-Cairo Wayland Widget System

License:        MIT
URL:            https://github.com/maxfridbe/WayWidget
Source0:        %{name}-%{version}.tar.gz

BuildRequires:  pkgconfig
BuildRequires:  wayland-devel
BuildRequires:  cairo-devel
BuildRequires:  cairo-gobject-devel
BuildRequires:  librsvg2-devel
BuildRequires:  libxkbcommon-devel
BuildRequires:  glib2-devel
BuildRequires:  pango-devel

%description
A lightweight Wayland widget system that renders SVG templates using Cairo and JavaScript.

%prep
%setup -q

%build
cd waywidget
cargo build --release

%install
mkdir -p %{buildroot}%{_bindir}
install -m 0755 waywidget/target/release/waywidget %{buildroot}%{_bindir}/waywidget

%files
%{_bindir}/waywidget

%changelog
* Wed Mar 04 2026 Max Fridbe <maxfridbe@gmail.com> - 0.1.0-1
- Initial release
