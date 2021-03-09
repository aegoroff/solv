# This script takes care of building your crate and packaging it for release

set -ex

main() {
  local src=$(pwd) \
  stage=

  case $TRAVIS_OS_NAME in
  linux)
    stage=$(mktemp -d)
    ;;
  osx)
    stage=$(mktemp -d -t tmp)
    ;;
  esac

  test -f Cargo.lock || cargo generate-lockfile

  cross build --manifest-path solv/Cargo.toml --target $TARGET --release

  if [ $TARGET = x86_64-pc-windows-gnu ]; then
    cp target/$TARGET/release/solv.exe $stage/
  else
    cp target/$TARGET/release/solv $stage/
  fi

  cp target/$TARGET/release/solv $stage/

  cd $stage
  tar czf $src/$CRATE_NAME-$TRAVIS_TAG-$TARGET.tar.gz *
  cd $src

  rm -rf $stage
}

main
