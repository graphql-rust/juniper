#! /usr/bin/env bash

# Usage: ./ci-build.sh VERSION
#
# This script builds the book to HTML with mdbook
# commits and pushes the contents to the repo in the "gh-pages" branch.
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
    curl -L https://github.com/rust-lang-nursery/mdBook/releases/download/v0.2.0/mdbook-v0.2.0-x86_64-unknown-linux-gnu.tar.gz | tar xzf -
    mv ./mdbook /tmp/mdbook
    MDBOOK="/tmp/mdbook"
fi

$MDBOOK build
echo $VERSION > ./_rendered/VERSION
rm -rf /tmp/book-content
mv ./_rendered /tmp/book-content

cd $DIR/../..
git clean -fd
git checkout gh-pages
rm -rf $VERSION
mv /tmp/book-content ./$VERSION
git remote set-url --push origin git@github.com:graphql-rust/juniper.git
git config --local user.name "Juniper Bot"
git config --local user.email "juniper@example.com"
git add -A $VERSION
git diff-index --quiet HEAD || git commit -m "Updated book for $VERSION ***NO_CI***"
git push origin gh-pages
