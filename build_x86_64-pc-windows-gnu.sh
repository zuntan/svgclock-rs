#!/bin/bash

# $ sudo apt install gcc-mingw-w64-x86-64-win32
# $ rustup target add aarch64-unknown-linux-gnu
# $ rustup target list --installed


GTK3_LIB=`realpath ./target/GTK3_Gvsbuild.x86_64-pc-windows`

export PKG_CONFIG_SYSROOT_DIR_x86_64_pc_windows_gnu=$GTK3_LIB
export PKG_CONFIG_PATH=$GTK3_LIB/lib/pkgconfig
export RUSTFLAGS="-L $GTK3_LIB/lib -C link-arg=-Wl,-subsystem,windows"

echo $PKG_CONFIG_LIBDIR
echo $PKG_CONFIG_SYSROOT_DIR_x86_64_pc_windows_gnu
echo $RUSTFLAGS

GTK_URL=https://github.com/wingtk/gvsbuild/releases/download/2025.8.0/GTK3_Gvsbuild_2025.8.0_x64.zip
GTK_ZIP=`basename $GTK_URL`

TARGET_EXE=target/x86_64-pc-windows-gnu/release/svgclock-rs.exe

ZIP_TARGET=`realpath ./target`
ZIP_DIR=$ZIP_TARGET/svgclock-rs.x86_64-pc-windows
ZIP_FILE=$ZIP_TARGET/svgclock-rs.zip

if [ ! -d $GTK3_LIB ]; then
    echo download $GTK_ZIP
    mkdir $GTK3_LIB
    ( cd $GTK3_LIB && wget $GTK_URL && unzip $GTK_ZIP )
else
    echo download skip
fi

cargo clean --release --target x86_64-pc-windows-gnu
cargo build --release --target x86_64-pc-windows-gnu
# cargo build           --target x86_64-pc-windows-gnu

if [ -d $ZIP_DIR ]; then
    rm -rf $ZIP_DIR
fi

mkdir $ZIP_DIR
mkdir $ZIP_DIR/theme

cp theme/clock_theme*svg $ZIP_DIR/theme

DLL_AND_EXE="
gspawn-win64-helper.exe
atk-1.0-0.dll
cairo-2.dll
cairo-gobject-2.dll
epoxy-0.dll
ffi-8.dll
fontconfig-1.dll
freetype-6.dll
fribidi-0.dll
gdk-3-vs17.dll
gdk_pixbuf-2.0-0.dll
gio-2.0-0.dll
glib-2.0-0.dll
gmodule-2.0-0.dll
gobject-2.0-0.dll
gtk-3-vs17.dll
harfbuzz.dll
intl.dll
jpeg62.dll
libexpat.dll
libpng16.dll
pango-1.0-0.dll
pangocairo-1.0-0.dll
pangoft2-1.0-0.dll
pangowin32-1.0-0.dll
pcre2-8-0.dll
pixman-1-0.dll
tiff.dll
xml2-16.dll
zlib1.dll
"

cp $TARGET_EXE $ZIP_DIR
( cd $GTK3_LIB/bin && tar c $DLL_AND_EXE ) | ( cd $ZIP_DIR && tar xv )
echo "xxx" $ZIP_FILE
echo `realpath --relative-base=$ZIP_TARGET $ZIP_DIR`
( cd $ZIP_TARGET && zip $ZIP_FILE `realpath --relative-base=$ZIP_TARGET $ZIP_DIR`/* -r )
