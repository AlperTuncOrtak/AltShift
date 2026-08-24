#!/usr/bin/env bash
# Fetches the word lists the accuracy harness trains and tests on.
#
# Source: hermitdave/FrequencyWords (MIT), derived from the OpenSubtitles
# corpus. Frequency-ordered real usage rather than a dictionary, which matters:
# we are modelling what people actually type, not what a lexicon admits.
#
# This script only strips the frequency column. Filtering by script happens in
# the Rust loader, where a `char` is a character -- BSD grep matches a Cyrillic
# range byte-wise, so `[а-яё]{3,}` silently counts bytes and lets two-letter
# words through.
#
# The lists are not committed: they are large, they are someone else's data,
# and fetching makes the build reproducible without vendoring them.
set -euo pipefail
cd "$(dirname "$0")"
mkdir -p data

base="https://raw.githubusercontent.com/hermitdave/FrequencyWords/master/content/2018"

for lang in en ru; do
    echo "fetching $lang..."
    curl -sSfL --max-time 60 "$base/$lang/${lang}_50k.txt" | cut -d' ' -f1 > "data/$lang.txt"
    echo "  data/$lang.txt: $(wc -l < "data/$lang.txt" | tr -d ' ') words"
done
