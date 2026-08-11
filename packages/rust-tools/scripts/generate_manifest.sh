#!/bin/bash
set -e

# Generate release artifact manifest
MANIFEST_FILE="release-manifest.json"
TARGET_DIR="../../target/release"

echo "{" > $MANIFEST_FILE
echo "  \"version\": \"$(cargo pkgid | cut -d# -f2 | cut -d: -f2)\"," >> $MANIFEST_FILE
echo "  \"timestamp\": \"$(date -u +'%Y-%m-%dT%H:%M:%SZ')\"," >> $MANIFEST_FILE
echo "  \"artifacts\": {" >> $MANIFEST_FILE

BINARIES=("relay-agent" "terminal-tool" "curl-tool" "searxng-search-tool")
FIRST=true

for bin in "${BINARIES[@]}"; do
    if [ -f "$TARGET_DIR/$bin" ]; then
        size=$(wc -c < "$TARGET_DIR/$bin" | tr -d ' ')
        if [ "$FIRST" = true ]; then
            FIRST=false
        else
            echo "    ," >> $MANIFEST_FILE
        fi
        sha256=$(sha256sum "$TARGET_DIR/$bin" | awk '{print $1}')
        echo "    \"$bin\": {" >> $MANIFEST_FILE
        echo "      \"size\": $size," >> $MANIFEST_FILE
        echo "      \"sha256\": \"$sha256\"" >> $MANIFEST_FILE
        echo "    }" >> $MANIFEST_FILE
    fi
done

echo "  }" >> $MANIFEST_FILE
echo "}" >> $MANIFEST_FILE

echo "Manifest generated at $MANIFEST_FILE"
