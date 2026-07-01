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
legacy_issue_version_re='^Version:[[:space:]]*v?0\.1\.0-issue-'
refs_re='^Refs[[:space:]]+#[0-9]+([[:space:],]+#[0-9]+)*$'
issue_19_version_re='^Version:[[:space:]]*v?[0-9]+\.[0-9]+\.[0-9]+-[0-9A-Za-z.-]*issue-19[0-9A-Za-z.-]*'
closing_issue_19_re='(^|[^[:alnum:]_])(close[sd]?|fix(e[sd])?|resolve[sd]?)[[:space:]]+#19([^0-9]|$)'

check_message() {
  local context="$1"
  local message="$2"
  local failed=0

  if ! grep -Eq "$version_re" <<<"$message"; then
    echo "::error::$context missing SemVer Version: trailer"
    failed=1
  fi

  if grep -Eq "$legacy_issue_version_re" <<<"$message"; then
    echo "::error::$context uses legacy 0.1.0 issue version; use an issue-slice SemVer such as 0.19.1-<slug>"
    failed=1
  fi

  if ! grep -Eq '^Deprecations:[[:space:]]*[^[:space:]].*$' <<<"$message"; then
    echo "::error::$context missing Deprecations: trailer"
    failed=1
  fi

  if ! grep -Eq "$refs_re" <<<"$message"; then
    echo "::error::$context missing non-closing Refs #issue trailer"
    failed=1
  fi

  if grep -Eq "$issue_19_version_re" <<<"$message"; then
    if ! grep -Eq '^Refs[[:space:]].*#19' <<<"$message" || ! grep -Eq '^Refs[[:space:]].*#305' <<<"$message"; then
      echo "::error::$context issue-19 versions must include Refs #19, #305"
      failed=1
    fi
  fi

  if grep -Eiq "$closing_issue_19_re" <<<"$message"; then
    echo "::error::$context must use Refs #19, not a close-style issue 19 keyword"
    failed=1
  fi

  return "$failed"
}

if [[ "$target" == "--text-file" ]]; then
  context="${2:-text file}"
  file="${3:-}"
  if [[ -z "$file" || ! -f "$file" ]]; then
    echo "::error::$context file not found"
    exit 1
  fi
  message="$(tr -d '\r' <"$file")"
  check_message "$context" "$message"
  exit "$?"
fi

for commit in "${commits[@]}"; do
  message="$(git log -1 --format=%B "$commit" | tr -d '\r')"
  subject="$(git log -1 --format=%s "$commit")"

  if ! check_message "commit $commit ($subject)" "$message"; then
    failed=1
  fi
done

exit "$failed"
