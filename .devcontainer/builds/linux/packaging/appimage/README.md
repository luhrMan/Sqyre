# Sqyre AppImage

Build from repo root: `.devcontainer/builds/linux/packaging/appimage/build-appimage.sh` (or from this dir: `./build-appimage.sh`).

The recipe follows the [AppImage excludelist](https://github.com/AppImage/pkg2appimage/blob/master/excludelist): we do **not** bundle `libGL`, `libGLX`, `libGLdispatch`, `libX11`, `libxcb`, `libX11-xcb`, `libstdc++`, `libgcc_s`, or `libz`; the host provides them. That avoids GLX/OpenGL conflicts on NixOS and other distros with non-standard library paths.

## NixOS: if you still see GLX errors

If you see "No GLXFBConfigs" or "Failed to find a suitable GLXFBConfig", ensure the host OpenGL stack is visible to the AppImage:

```bash
LD_LIBRARY_PATH="/run/opengl-driver/lib:$LD_LIBRARY_PATH" appimage-run ./Sqyre-*.AppImage
```

Alternatively use **nixGL**: `nixGL appimage-run ./Sqyre-*.AppImage`.
