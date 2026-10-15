#!/usr/bin/env bash
# Compares each pinned Kora image with upstream's latest release of its own channel:
# kora/Dockerfile (the default, pre-release), kora/Dockerfile.public (the public relayer, same
# channel) and kora/Dockerfile.stable (the alternative).
#
#   ./scripts/check-upstream.sh
#
# Uses only curl and standard tools against the public GitHub and container registry APIs.
# No account or token of yours is needed (the registry hands out an anonymous pull token).
# Exit code: 0 when every pin is the latest, 1 when any differs, 2 when upstream could
# not be read. A newer release is a prompt to read docs/updating-from-upstream.md, not to bump.

set -u
cd "$(dirname "$0")/.." || exit 2

REPO=solana-foundation/kora
ACCEPT='application/vnd.oci.image.index.v1+json, application/vnd.docker.distribution.manifest.list.v2+json, application/vnd.oci.image.manifest.v1+json, application/vnd.docker.distribution.manifest.v2+json'

# "tag prerelease" per line, newest first, server releases only (the repo also tags its SDK).
releases="$(curl -s -m 20 "https://api.github.com/repos/$REPO/releases?per_page=100" \
  | grep -E '"(tag_name|prerelease)":' | paste - - \
  | sed -E 's/.*"tag_name": *"([^"]*)".*"prerelease": *(true|false).*/\1 \2/' | grep -E '^v[0-9]')"
if [ -z "$releases" ]; then
  echo "could not read the release list from GitHub (rate limit or network)" >&2
  exit 2
fi
token="$(curl -s -m 20 "https://ghcr.io/token?scope=repository:$REPO:pull" | sed -E 's/.*"token":"([^"]*)".*/\1/')"

digest_of() { # tag -> digest of its image index
  curl -sI -m 20 -H "Authorization: Bearer $token" -H "Accept: $ACCEPT" \
    "https://ghcr.io/v2/$REPO/manifests/$1" | tr -d '\r' | sed -n 's/^[Dd]ocker-[Cc]ontent-[Dd]igest: *//p'
}

status=0
compare() { # label, dockerfile, latest tag
  pinned="$(sed -n -E 's/^FROM .*kora:([^@]+)@(sha256:[0-9a-f]+).*/\1 \2/p' "$2")"
  pinned_tag="${pinned% *}"; pinned_digest="${pinned#* }"
  latest_digest="$(digest_of "$3")"
  echo "$1 ($2)"
  echo "  pinned  $pinned_tag  $pinned_digest"
  echo "  latest  $3  ${latest_digest:-digest not readable}"
  if [ -z "$latest_digest" ]; then
    echo "  UNKNOWN: the registry did not answer"; status=2
  elif [ "$pinned_tag" = "$3" ] && [ "$pinned_digest" = "$latest_digest" ]; then
    echo "  OK: up to date"
  else
    echo "  DIFFERENT: read docs/updating-from-upstream.md"; [ "$status" -eq 0 ] && status=1
  fi
}

latest_prerelease="$(printf '%s\n' "$releases" | awk '$2=="true"{print $1; exit}')"
compare "pre-release (default)" kora/Dockerfile "$latest_prerelease"
compare "pre-release (public relayer)" kora/Dockerfile.public "$latest_prerelease"
compare "stable (alternative)" kora/Dockerfile.stable "$(printf '%s\n' "$releases" | awk '$2=="false"{print $1; exit}')"
exit "$status"
