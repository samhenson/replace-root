
# replace-root

This utility replaces the contents of / with the contents of /new_root.  This allows installation of a custom root filesystem on a VPS or VM provider that does not support multiple disks or installation from an ISO.  The binary should be copied to /sbin/init, and will perform the replacement on the next system boot.

## Possible Improvements

- In an attempt to cleanly unmount the filesystem, the executable and dynamic libraries are left in /tmp, and should be deleted by most distro initscripts on the next reboot.  This could be avoided by copying the executable to a tmpfs filesystem and re-execing.
- The need to copy libraries in addition to the executable could be eliminated by statically linking against musl.

