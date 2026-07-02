#!/usr/bin/env bash
set -euo pipefail

target="${1:-HEAD}"
failed=0

version_re='^Version:[[:space:]]*v?[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?(\+[0-9A-Za-z.-]+)?$'
refs_re='^Refs[[:space:]]+#[0-9]+([[:space:],]+#[0-9]+)*$'
issue_19_version_re='^Version:[[:space:]]*v?[0-9]+\.[0-9]+\.[0-9]+-[0-9A-Za-z.-]*issue-19[0-9A-Za-z.-]*'
closing_issue_re='(^|[^[:alnum:]_])(close[sd]?|fix(e[sd])?|resolve[sd]?)[[:space:]:]+#[0-9]+([^0-9]|$)'
polluted_payload_re='(BEGIN[[:space:]]+(SECRET|RSA[[:space:]]+PRIVATE[[:space:]]+KEY|PRIVATE[[:space:]]+KEY)|END[[:space:]]+SECRET|raw_payload_included[[:space:]]*[:=][[:space:]]*true|raw_prompt[[:space:]]*[:=]|password[[:space:]]*=|api[_-]?key[[:space:]]*=|sk-[A-Za-z0-9_-]{16,})'

trim() {
  sed -E 's/^[[:space:]]+//; s/[[:space:]]+$//'
}

version_base() {
  sed -E 's/^v?([0-9]+\.[0-9]+\.[0-9]+).*/\1/' <<<"$1"
}

current_package_version() {
  sed -nE 's/^Current package version:[[:space:]]*`?([^`[:space:]]+)`?.*$/\1/p' VERSION_LEDGER.md | head -n 1
}

ledger_status_for_version() {
  local wanted="$1"

  awk -F'|' -v wanted="$wanted" '
    function trim_field(value) {
      gsub(/^[ \t]+|[ \t]+$/, "", value)
      gsub(/`/, "", value)
      return value
    }
    /^\|[ \t]*(active|retired)[ \t]*\|/ {
      status = trim_field($2)
      version = trim_field($3)
      if (version == wanted) {
        print status
        found = 1
      }
    }
    END { exit found ? 0 : 1 }
  ' VERSION_LEDGER.md
}

check_version_ledger() {
  local current

  if [[ ! -f VERSION_LEDGER.md ]]; then
    echo "::error::VERSION_LEDGER.md is required"
    failed=1
    return
  fi

  current="$(current_package_version || true)"
  if [[ ! "$current" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "::error::VERSION_LEDGER.md missing Current package version SemVer"
    failed=1
  elif [[ "$current" == "0.1.0" ]]; then
    echo "::error::VERSION_LEDGER.md current package version must not be retired 0.1.0"
    failed=1
  fi

  if ! awk -F'|' -v current="$current" '
    function trim_field(value) {
      gsub(/^[ \t]+|[ \t]+$/, "", value)
      gsub(/`/, "", value)
      return value
    }
    function version_base(value) {
      sub(/^v?/, "", value)
      sub(/-.*/, "", value)
      sub(/\+.*/, "", value)
      return value
    }
    function fail(message) {
      printf "::error::VERSION_LEDGER.md %s\n", message
      failed = 1
    }
    /^\|[ \t]*(active|retired)[ \t]*\|/ {
      status = trim_field($2)
      version = trim_field($3)
      scope = trim_field($4)
      deprecations = trim_field($5)
      refs = trim_field($6)
      rows++

      if (seen[version]++) {
        fail("reuses Version: " version)
      }
      if (version !~ /^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?(\+[0-9A-Za-z.-]+)?$/) {
        fail("has invalid Version: " version)
      }
      if (scope == "" || deprecations == "" || refs == "") {
        fail("row for " version " must have scope, deprecations, and refs")
      }
      if (refs !~ /^#[0-9]+([[:space:],]+#[0-9]+)*$/) {
        fail("row for " version " has invalid Refs: " refs)
      }

      base = version_base(version)
      if (status == "active") {
        active_count++
        if (base != current) {
          fail("active Version: " version " must match current package version " current)
        }
      }
      if (base == "0.1.0" && status != "retired") {
        fail("0.1.0 Version: " version " must be retired")
      }
      if (status == "retired" && base == current) {
        fail("retired Version: " version " must not use current package version " current)
      }
    }
    END {
      if (rows == 0) {
        fail("needs at least one ledger row")
      }
      if (active_count != 1) {
        fail("needs exactly one active ledger row")
      }
      exit failed ? 1 : 0
    }
  ' VERSION_LEDGER.md; then
    failed=1
  fi
}

check_message() {
  local context="$1"
  local message="$2"
  local mode="${3:-single}"
  local local_failed=0
  local version_lines
  local valid_version_lines
  local version_count
  local deprecation_lines
  local deprecation_count

  version_lines="$(grep -E '^Version:' <<<"$message" || true)"
  valid_version_lines="$(grep -E "$version_re" <<<"$message" || true)"
  version_count="$(grep -Ec "$version_re" <<<"$message" || true)"

  if [[ "$version_count" -eq 0 ]]; then
    echo "::error::$context missing SemVer Version: trailer"
    local_failed=1
  fi

  if [[ "$(grep -Ec '^Version:' <<<"$version_lines" || true)" -ne "$version_count" ]]; then
    echo "::error::$context contains an invalid or empty Version: trailer"
    local_failed=1
  fi

  if [[ "$mode" == "single" && "$version_count" -ne 1 ]]; then
    echo "::error::$context must contain exactly one Version: trailer"
    local_failed=1
  fi

  while IFS= read -r line; do
    local version
    local base
    local status

    [[ -z "$line" ]] && continue
    version="$(sed -E 's/^Version:[[:space:]]*v?//' <<<"$line")"
    base="$(version_base "$version")"
    status="$(ledger_status_for_version "$version" || true)"

    if [[ -z "$status" ]]; then
      echo "::error::$context Version: $version is missing from VERSION_LEDGER.md"
      local_failed=1
    elif [[ "$context" == commit\ * && "$base" == "0.1.0" ]]; then
      echo "::error::$context Version: $version uses retired 0.1.0 scaffold version"
      local_failed=1
    elif [[ "$base" == "0.1.0" && "$status" != "retired" ]]; then
      echo "::error::$context Version: $version uses retired 0.1.0 but is not marked retired"
      local_failed=1
    fi
  done <<<"$valid_version_lines"

  deprecation_lines="$(grep -E '^Deprecations:' <<<"$message" || true)"
  deprecation_count="$(grep -Ec '^Deprecations:[[:space:]]*[^[:space:]].*$' <<<"$message" || true)"

  if [[ "$deprecation_count" -eq 0 ]]; then
    echo "::error::$context missing Deprecations: trailer"
    local_failed=1
  fi

  if [[ "$(grep -Ec '^Deprecations:' <<<"$deprecation_lines" || true)" -ne "$deprecation_count" ]]; then
    echo "::error::$context contains an invalid or empty Deprecations: trailer"
    local_failed=1
  fi

  if [[ "$mode" == "single" && "$deprecation_count" -ne 1 ]]; then
    echo "::error::$context must contain exactly one Deprecations: trailer"
    local_failed=1
  fi

  if [[ "$mode" != "single" && "$version_count" -ne "$deprecation_count" ]]; then
    echo "::error::$context Version: and Deprecations: ledger counts must match"
    local_failed=1
  fi

  if ! grep -Eq "$refs_re" <<<"$message"; then
    echo "::error::$context missing non-closing Refs #issue trailer"
    local_failed=1
  fi

  if grep -Eq "$issue_19_version_re" <<<"$message"; then
    if ! grep -Eq '^Refs[[:space:]].*#19' <<<"$message" || ! grep -Eq '^Refs[[:space:]].*#305' <<<"$message"; then
      echo "::error::$context issue-19 versions must include Refs #19, #305"
      local_failed=1
    fi
  fi

  if grep -Eiq "$closing_issue_re" <<<"$message"; then
    echo "::error::$context must use Refs #issue, not a close-style issue keyword"
    local_failed=1
  fi

  if grep -Eiq "$polluted_payload_re" <<<"$message"; then
    echo "::error::$context contains #305 polluted payload marker; use digest-only evidence"
    local_failed=1
  fi

  return "$local_failed"
}

first_party_packages() {
  git ls-files '*Cargo.toml' | while IFS= read -r manifest; do
    awk '
      /^\[package\]/ { in_package = 1; next }
      /^\[/ { in_package = 0 }
      in_package && /^name[[:space:]]*=/ {
        gsub(/"/, "", $3)
        print $3
        exit
      }
    ' "$manifest"
  done | sort -u
}

check_manifest_versions() {
  local current
  local manifest
  local package_version

  current="$(current_package_version || true)"
  [[ -z "$current" ]] && return

  while IFS= read -r manifest; do
    package_version="$(
      awk '
        /^\[package\]/ { in_package = 1; next }
        /^\[/ { in_package = 0 }
        in_package && /^version[[:space:]]*=/ {
          gsub(/"/, "", $3)
          print $3
          exit
        }
      ' "$manifest"
    )"

    if [[ -n "$package_version" && "$package_version" != "$current" ]]; then
      echo "::error::$manifest package version $package_version must match VERSION_LEDGER.md current package version $current"
      failed=1
    fi
  done < <(git ls-files '*Cargo.toml')
}

check_lock_versions() {
  local current
  local lock
  local package
  local lock_version

  current="$(current_package_version || true)"
  [[ -z "$current" ]] && return

  while IFS= read -r lock; do
    while IFS= read -r package; do
      [[ -z "$package" ]] && continue
      lock_version="$(
        awk -v package="$package" '
          function flush_package() {
            if (name == package) {
              print version
              found = 1
            }
          }
          /^\[\[package\]\]/ {
            flush_package()
            name = ""
            version = ""
            next
          }
          /^name[[:space:]]*=/ {
            name = $3
            gsub(/"/, "", name)
            next
          }
          /^version[[:space:]]*=/ {
            version = $3
            gsub(/"/, "", version)
            next
          }
          END { flush_package() }
        ' "$lock" | head -n 1
      )"

      if [[ -n "$lock_version" && "$lock_version" != "$current" ]]; then
        echo "::error::$lock package $package version $lock_version must match $current"
        failed=1
      fi
    done < <(first_party_packages)
  done < <(git ls-files '*Cargo.lock')
}

check_citation_version() {
  local current
  local citation_version

  [[ -f CITATION.cff ]] || return
  current="$(current_package_version || true)"
  [[ -z "$current" ]] && return
  citation_version="$(sed -nE 's/^version:[[:space:]]*"?([^"]+)"?$/\1/p' CITATION.cff | head -n 1)"

  if [[ -n "$citation_version" && "$citation_version" != "$current" ]]; then
    echo "::error::CITATION.cff version $citation_version must match $current"
    failed=1
  fi
}

check_latest_text_version_matches_current() {
  local context="$1"
  local message="$2"
  local current
  local latest_version
  local latest_base

  current="$(current_package_version || true)"
  latest_version="$(grep -E "$version_re" <<<"$message" | tail -n 1 | sed -E 's/^Version:[[:space:]]*v?//' || true)"
  [[ -z "$current" || -z "$latest_version" ]] && return
  latest_base="$(version_base "$latest_version")"

  if [[ "$latest_base" != "$current" ]]; then
    echo "::error::$context latest Version: $latest_version must match current package version $current"
    failed=1
  fi
}

check_latest_commit_version_matches_current() {
  local latest_commit="$1"
  local latest_message

  latest_message="$(git log -1 --format=%B "$latest_commit" | tr -d '\r')"
  check_latest_text_version_matches_current "latest commit $latest_commit" "$latest_message"
}

check_commit_versions_unique() {
  local commit
  declare -A seen_versions=()

  for commit in "$@"; do
    local message
    local subject
    local version

    message="$(git log -1 --format=%B "$commit" | tr -d '\r')"
    subject="$(git log -1 --format=%s "$commit")"
    version="$(grep -E "$version_re" <<<"$message" | sed -E 's/^Version:[[:space:]]*v?//' || true)"

    [[ -z "$version" || "$version" == *$'\n'* ]] && continue

    if [[ -n "${seen_versions[$version]:-}" ]]; then
      echo "::error::commit $commit ($subject) reuses Version: $version already used by ${seen_versions[$version]}"
      failed=1
    else
      seen_versions[$version]="$commit"
    fi
  done
}

run_repo_version_checks() {
  check_version_ledger
  check_manifest_versions
  check_lock_versions
  check_citation_version
}

if [[ "$target" == "--text-file" || "$target" == "--text-file-ledger-only" ]]; then
  context="${2:-text file}"
  file="${3:-}"
  if [[ -z "$file" || ! -f "$file" ]]; then
    echo "::error::$context file not found"
    exit 1
  fi

  message="$(tr -d '\r' <"$file")"
  run_repo_version_checks
  if ! check_message "$context" "$message" "multi"; then
    failed=1
  fi
  check_latest_text_version_matches_current "$context" "$message"
  exit "$failed"
fi

if [[ "$target" == *..* ]]; then
  mapfile -t commits < <(git rev-list --reverse "$target")
else
  commits=("$target")
fi

if [[ "${#commits[@]}" -eq 0 ]]; then
  echo "::error::no commits found for $target"
  exit 1
fi

run_repo_version_checks

for commit in "${commits[@]}"; do
  message="$(git log -1 --format=%B "$commit" | tr -d '\r')"
  subject="$(git log -1 --format=%s "$commit")"

  if ! check_message "commit $commit ($subject)" "$message" "single"; then
    failed=1
  fi
done

check_commit_versions_unique "${commits[@]}"
latest_index=$((${#commits[@]} - 1))
check_latest_commit_version_matches_current "${commits[$latest_index]}"

exit "$failed"
