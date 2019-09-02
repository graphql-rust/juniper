#! /usr/bin/env bash

# Usage: ./ci-build.sh VERSION
#
# This script builds the book to HTML with mdbook
# commits and pushes the contents to the repo in the "book-zh" branch.
#
# It is only inteded for use on the CI!

# Enable strict error checking.
set -exo pipefail

DIR=$(dirname $(readlink -f $0))
MDBOOK="mdbook"

cd $DIR

# Verify version argument.

if [[ -z "$1" ]]; then
    echo "Missing required argument 'version': cargo make build-book VERSION"
    exit
fi
VERSION="$1"

# Download mdbook if not found.

if [ $MDBOOK -h ]; then
    echo "mdbook found..."
else
    echo "mdbook not found. Downloading..."
    curl -L https://github.com/rust-lang-nursery/mdBook/releases/download/v0.3.1/mdbook-v0.3.1-x86_64-unknown-linux-gnu.tar.gz | tar xzf -
    mv ./mdbook /tmp/mdbook
    MDBOOK="/tmp/mdbook"
fi

$MDBOOK build
echo $VERSION > ./docs/VERSION
rm -rf /tmp/book-content
mv ./docs /tmp/book-content

cd $DIR/../..
git clean -fd
git checkout book-zh
rm -rf $VERSION
mv /tmp/book-content ./$VERSION
git remote set-url --push origin git@github.com:zzy/juniper.git
git config --local user.name "zzy"
git config --local user.email "9809920@qq.com"
git add -A $VERSION
git diff-index --quiet HEAD || git commit -m "Updated book for $VERSION ***NO_CI***"
git push origin book-zh
