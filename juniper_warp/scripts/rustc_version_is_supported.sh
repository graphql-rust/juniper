# Exits successfully if the Rust version that is running is compatible with juniper_warp.
# This must be run in cargo make to have the proper environment variables.

MINOR_VERSION=`echo $CARGO_MAKE_RUST_VERSION | cut -d. -f2`;
if (( $MINOR_VERSION > 22 )); then
  exit 0
else
  exit 1
fi
