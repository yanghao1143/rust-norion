#!/usr/bin/env bash
set -euo pipefail

target="${1:-HEAD}"

if [[ "$target" == *..* ]]; then
  mapfile -t commits < <(git rev-list --reverse "$target")
else
  commits=("$target")
fi

if [[ "${#commits[@]}" -eq 0 ]]; then
  echo "::error::no commits found for $target"
  exit 1
fi

failed=0
version_re='^Version:[[:space:]]*v?[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?(\+[0-9A-Za-z.-]+)?$'

for commit in "${commits[@]}"; do
  message="$(git log -1 --format=%B "$commit" | tr -d '\r')"
  subject="$(git log -1 --format=%s "$commit")"

  if ! grep -Eq "$version_re" <<<"$message"; then
    echo "::error::commit $commit ($subject) missing SemVer Version: trailer"
    failed=1
  fi

  if ! grep -Eq '^Deprecations:[[:space:]]*[^[:space:]].*$' <<<"$message"; then
    echo "::error::commit $commit ($subject) missing Deprecations: trailer"
    failed=1
  fi
done

exit "$failed"
