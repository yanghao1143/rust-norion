#!/usr/bin/env bash
set -euo pipefail

target="${1:-HEAD}"
version_ledger_file="${VERSION_LEDGER_FILE:-VERSION_LEDGER.md}"

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
ledger_version_re='^v?[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?(\+[0-9A-Za-z.-]+)?$'
retired_version_re='^Version:[[:space:]]*v?0\.1\.0([[:space:]]|[-+]|$)'
retired_ledger_version_re='^v?0\.1\.0([[:space:]]|[-+]|$)'
refs_re='^Refs[[:space:]]+#[0-9]+([[:space:],]+#[0-9]+)*$'
ledger_refs_re='^#[0-9]+([[:space:]]*,[[:space:]]*#[0-9]+)*$'
issue_19_version_re='^Version:[[:space:]]*v?[0-9]+\.[0-9]+\.[0-9]+-[0-9A-Za-z.-]*issue-19[0-9A-Za-z.-]*'
closing_issue_19_re='(^|[^[:alnum:]_])(close[sd]?|fix(e[sd])?|resolve[sd]?)[[:space:]]+#19([^0-9]|$)'

check_version_line_order() {
  local context="$1"
  local lines="$2"
  local previous_version=""
  local previous_major=0
  local previous_minor=0
  local previous_patch=0
  local failed=0

  declare -A seen_versions=()

  while IFS= read -r line; do
    local version

    [[ -z "$line" ]] && continue
    version="$(sed -E 's/^Version:[[:space:]]*v?//' <<<"$line")"

    if [[ -n "${seen_versions[$version]:-}" ]]; then
      echo "::error::$context reuses Version: $version in text ledger"
      failed=1
    else
      seen_versions[$version]=1
    fi

    if [[ "$version" =~ ^([0-9]+)\.([0-9]+)\.([0-9]+) ]]; then
      local major="${BASH_REMATCH[1]}"
      local minor="${BASH_REMATCH[2]}"
      local patch="${BASH_REMATCH[3]}"

      if [[ -n "$previous_version" ]]; then
        if (( major < previous_major || \
              (major == previous_major && minor < previous_minor) || \
              (major == previous_major && minor == previous_minor && patch <= previous_patch) )); then
          echo "::error::$context Version: $version must be greater than previous Version: $previous_version"
          failed=1
        fi
      fi

      previous_version="$version"
      previous_major="$major"
      previous_minor="$minor"
      previous_patch="$patch"
    fi
  done <<<"$lines"

  return "$failed"
}

check_message() {
  local context="$1"
  local message="$2"
  local mode="${3:-multi}"
  local failed=0
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
    failed=1
  fi

  if [[ "$(grep -Ec '^Version:' <<<"$version_lines" || true)" -ne "$version_count" ]]; then
    echo "::error::$context contains an invalid or empty Version: trailer"
    failed=1
  fi

  if grep -Eq "$retired_version_re" <<<"$message"; then
    echo "::error::$context uses retired 0.1.0 version; use an issue-slice SemVer such as 0.19.39-<slug>"
    failed=1
  fi

  if [[ "$mode" == "single" && "$version_count" -ne 1 ]]; then
    echo "::error::$context must contain exactly one Version: trailer"
    failed=1
  fi

  deprecation_lines="$(grep -E '^Deprecations:' <<<"$message" || true)"
  deprecation_count="$(grep -Ec '^Deprecations:[[:space:]]*[^[:space:]].*$' <<<"$message" || true)"

  if [[ "$deprecation_count" -eq 0 ]]; then
    echo "::error::$context missing Deprecations: trailer"
    failed=1
  fi

  if [[ "$(grep -Ec '^Deprecations:' <<<"$deprecation_lines" || true)" -ne "$deprecation_count" ]]; then
    echo "::error::$context contains an invalid or empty Deprecations: trailer"
    failed=1
  fi

  if [[ "$mode" == "single" && "$deprecation_count" -ne 1 ]]; then
    echo "::error::$context must contain exactly one Deprecations: trailer"
    failed=1
  fi

  if [[ "$mode" != "single" && "$version_count" -ne "$deprecation_count" ]]; then
    echo "::error::$context Version: and Deprecations: ledger counts must match"
    failed=1
  fi

  if [[ "$mode" != "single" && "$version_count" -gt 0 ]]; then
    if ! check_version_line_order "$context" "$valid_version_lines"; then
      failed=1
    fi
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

ledger_has_version_deprecation() {
  local version="$1"
  local deprecation="$2"

  awk -v version="$version" -v deprecation="$deprecation" '
    function trim(value) {
      gsub(/^[[:space:]]+|[[:space:]]+$/, "", value)
      return value
    }
    BEGIN { FS = "|" }
    /^\|/ {
      if (trim($2) == version) {
        found_version = 1
        if (trim($3) == deprecation) {
          found = 1
        }
      }
    }
    END {
      if (found) {
        exit 0
      }
      if (found_version) {
        exit 2
      }
      exit 1
    }
  ' "$version_ledger_file"
}

check_version_ledger_entries() {
  local context="$1"
  local message="$2"
  local failed=0
  local index

  if [[ ! -f "$version_ledger_file" ]]; then
    echo "::error::$context missing $version_ledger_file"
    return 1
  fi

  mapfile -t ledger_versions < <(grep -E "$version_re" <<<"$message" | sed -E 's/^Version:[[:space:]]*v?//')
  mapfile -t ledger_deprecations < <(grep -E '^Deprecations:[[:space:]]*[^[:space:]].*$' <<<"$message" | sed -E 's/^Deprecations:[[:space:]]*//')

  if [[ "${#ledger_versions[@]}" -ne "${#ledger_deprecations[@]}" ]]; then
    return 0
  fi

  for index in "${!ledger_versions[@]}"; do
    if ledger_has_version_deprecation "${ledger_versions[$index]}" "${ledger_deprecations[$index]}"; then
      continue
    fi

    case "$?" in
      2)
        echo "::error::$context $version_ledger_file has Version: ${ledger_versions[$index]} with a different Deprecations value"
        ;;
      *)
        echo "::error::$context missing $version_ledger_file entry for Version: ${ledger_versions[$index]}"
        ;;
    esac
    failed=1
  done

  return "$failed"
}

check_manifest_versions() {
  local manifest

  while IFS= read -r manifest; do
    if awk '
      /^\[package\]/ { in_package = 1; next }
      /^\[/ { in_package = 0 }
      in_package && /^version[[:space:]]*=[[:space:]]*"0\.1\.0"/ { found = 1 }
      END { exit found ? 0 : 1 }
    ' "$manifest"; then
      echo "::error::$manifest uses retired Cargo package version 0.1.0"
      failed=1
    fi
  done < <(git ls-files '*Cargo.toml')
}

check_lock_versions() {
  local lock
  local package

  while IFS= read -r lock; do
    while IFS= read -r package; do
      [[ -z "$package" ]] && continue
      if awk -v package="$package" '
        function check_package() {
          if (name == package && version == "0.1.0") {
            found = 1
          }
        }
        /^\[\[package\]\]/ {
          check_package()
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
        END {
          check_package()
          exit found ? 0 : 1
        }
      ' "$lock"; then
        echo "::error::$lock package $package uses retired Cargo.lock version 0.1.0"
        failed=1
      fi
    done < <(git ls-files '*Cargo.toml' | while IFS= read -r manifest; do
      awk '
        /^\[package\]/ { in_package = 1; next }
        /^\[/ { in_package = 0 }
        in_package && /^name[[:space:]]*=/ {
          gsub(/"/, "", $3)
          print $3
          exit
        }
      ' "$manifest"
    done | sort -u)
  done < <(git ls-files '*Cargo.lock')
}

root_package_version() {
  awk '
    /^\[package\]/ { in_package = 1; next }
    /^\[/ { in_package = 0 }
    in_package && /^version[[:space:]]*=/ {
      gsub(/"/, "", $3)
      print $3
      exit
    }
  ' Cargo.toml
}

trim_field() {
  sed -E 's/^[[:space:]]+//; s/[[:space:]]+$//' <<<"$1"
}

base_semver() {
  sed -E 's/^v?//; s/^([0-9]+\.[0-9]+\.[0-9]+).*/\1/' <<<"$1"
}

check_version_ledger_file() {
  local row_count=0
  local line_number=0
  local previous_version=""
  local previous_major=0
  local previous_minor=0
  local previous_patch=0
  local latest_version=""
  local latest_base_version
  local root_version

  if [[ ! -f "$version_ledger_file" ]]; then
    echo "::error::missing $version_ledger_file"
    failed=1
    return
  fi

  declare -A seen_ledger_versions=()

  while IFS= read -r line; do
    local version
    local deprecations
    local refs
    local major
    local minor
    local patch

    line_number=$((line_number + 1))
    [[ "$line" =~ ^\|[[:space:]]*Version[[:space:]]*\| ]] && continue
    [[ "$line" =~ ^\|[[:space:]]*--- ]] && continue
    [[ "$line" =~ ^\| ]] || continue

    IFS='|' read -r _ version deprecations refs _ <<<"$line"
    version="$(trim_field "${version:-}")"
    deprecations="$(trim_field "${deprecations:-}")"
    refs="$(trim_field "${refs:-}")"
    row_count=$((row_count + 1))

    if [[ -z "$version" || ! "$version" =~ $ledger_version_re ]]; then
      echo "::error::$version_ledger_file:$line_number has invalid Version value"
      failed=1
      continue
    fi

    if [[ "$version" =~ $retired_ledger_version_re ]]; then
      echo "::error::$version_ledger_file:$line_number uses retired 0.1.0 version"
      failed=1
    fi

    if [[ -z "$deprecations" ]]; then
      echo "::error::$version_ledger_file:$line_number missing Deprecations value"
      failed=1
    fi

    if [[ -z "$refs" || ! "$refs" =~ $ledger_refs_re ]]; then
      echo "::error::$version_ledger_file:$line_number missing issue Refs value"
      failed=1
    fi

    if [[ -n "${seen_ledger_versions[$version]:-}" ]]; then
      echo "::error::$version_ledger_file:$line_number reuses Version: $version"
      failed=1
    else
      seen_ledger_versions[$version]="$line_number"
    fi

    if [[ "$version" =~ ^v?([0-9]+)\.([0-9]+)\.([0-9]+) ]]; then
      major="${BASH_REMATCH[1]}"
      minor="${BASH_REMATCH[2]}"
      patch="${BASH_REMATCH[3]}"

      if [[ -n "$previous_version" ]]; then
        if (( major < previous_major || \
              (major == previous_major && minor < previous_minor) || \
              (major == previous_major && minor == previous_minor && patch <= previous_patch) )); then
          echo "::error::$version_ledger_file:$line_number Version: $version must be greater than previous Version: $previous_version"
          failed=1
        fi
      fi

      previous_version="$version"
      previous_major="$major"
      previous_minor="$minor"
      previous_patch="$patch"
    fi

    latest_version="$version"
  done <"$version_ledger_file"

  if [[ "$row_count" -eq 0 ]]; then
    echo "::error::$version_ledger_file has no version rows"
    failed=1
    return
  fi

  root_version="$(root_package_version)"
  latest_base_version="$(base_semver "$latest_version")"
  if [[ -n "$root_version" && "$latest_base_version" != "$root_version" ]]; then
    echo "::error::$version_ledger_file latest Version: $latest_version must match Cargo.toml package version $root_version"
    failed=1
  fi
}

check_active_metadata_versions() {
  local root_version
  local citation_version

  root_version="$(root_package_version)"
  [[ -z "$root_version" ]] && return 0

  if [[ -f CITATION.cff ]]; then
    citation_version="$(
      awk '
        /^version:[[:space:]]*/ {
          value = $0
          sub(/^version:[[:space:]]*/, "", value)
          gsub(/"/, "", value)
          gsub(/[[:space:]]/, "", value)
          print value
          exit
        }
      ' CITATION.cff
    )"

    if [[ "$citation_version" == "0.1.0" ]]; then
      echo "::error::CITATION.cff uses retired software version 0.1.0"
      failed=1
    elif [[ -n "$citation_version" && "$citation_version" != "$root_version" ]]; then
      echo "::error::CITATION.cff version $citation_version must match Cargo.toml package version $root_version"
      failed=1
    fi
  fi

  if [[ -f ROADMAP.md ]] && grep -Eq 'Cargo package version remains `?0\.1\.0`?' ROADMAP.md; then
    echo "::error::ROADMAP.md contains retired Cargo package version 0.1.0 status text"
    failed=1
  fi
}

check_manifest_versions_match_latest_commit() {
  local latest_commit_index
  local latest_commit
  local latest_message
  local latest_version
  local latest_package_version
  local manifest

  latest_commit_index=$((${#commits[@]} - 1))
  latest_commit="${commits[$latest_commit_index]}"
  latest_message="$(git log -1 --format=%B "$latest_commit" | tr -d '\r')"
  latest_version="$(grep -E "$version_re" <<<"$latest_message" | sed -E 's/^Version:[[:space:]]*v?//; s/^([0-9]+\.[0-9]+\.[0-9]+).*/\1/' || true)"

  [[ -z "$latest_version" || "$latest_version" == *$'\n'* ]] && return 0

  while IFS= read -r manifest; do
    latest_package_version="$(
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

    if [[ -n "$latest_package_version" && "$latest_package_version" != "$latest_version" ]]; then
      echo "::error::$manifest package version $latest_package_version must match latest commit Version: $latest_version"
      failed=1
    fi
  done < <(git ls-files '*Cargo.toml')
}

check_text_file_version_matches_manifest_versions() {
  local context="$1"
  local message="$2"
  local latest_version
  local latest_package_version
  local manifest

  latest_version="$(grep -E "$version_re" <<<"$message" | tail -n 1 | sed -E 's/^Version:[[:space:]]*v?//; s/^([0-9]+\.[0-9]+\.[0-9]+).*/\1/' || true)"

  [[ -z "$latest_version" ]] && return 0

  while IFS= read -r manifest; do
    latest_package_version="$(
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

    if [[ -n "$latest_package_version" && "$latest_package_version" != "$latest_version" ]]; then
      echo "::error::$context latest Version: $latest_version must match $manifest package version $latest_package_version"
      failed=1
    fi
  done < <(git ls-files '*Cargo.toml')
}

check_commit_version_order() {
  local previous_version=""
  local previous_major=0
  local previous_minor=0
  local previous_patch=0
  local commit

  declare -A seen_versions=()

  for commit in "${commits[@]}"; do
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

    if [[ "$version" =~ ^([0-9]+)\.([0-9]+)\.([0-9]+) ]]; then
      local major="${BASH_REMATCH[1]}"
      local minor="${BASH_REMATCH[2]}"
      local patch="${BASH_REMATCH[3]}"

      if [[ -n "$previous_version" ]]; then
        if (( major < previous_major || \
              (major == previous_major && minor < previous_minor) || \
              (major == previous_major && minor == previous_minor && patch <= previous_patch) )); then
          echo "::error::commit $commit ($subject) Version: $version must be greater than previous Version: $previous_version"
          failed=1
        fi
      fi

      previous_version="$version"
      previous_major="$major"
      previous_minor="$minor"
      previous_patch="$patch"
    fi
  done
}

if [[ "$target" == "--text-file" ]]; then
  context="${2:-text file}"
  file="${3:-}"
  if [[ -z "$file" || ! -f "$file" ]]; then
    echo "::error::$context file not found"
    exit 1
  fi
  message="$(tr -d '\r' <"$file")"
  if ! check_message "$context" "$message" "multi"; then
    failed=1
  fi
  if ! check_version_ledger_entries "$context" "$message"; then
    failed=1
  fi
  check_manifest_versions
  check_lock_versions
  check_active_metadata_versions
  check_version_ledger_file
  check_text_file_version_matches_manifest_versions "$context" "$message"
  exit "$failed"
fi

if [[ "$target" == "--text-file-ledger-only" ]]; then
  context="${2:-text file}"
  file="${3:-}"
  if [[ -z "$file" || ! -f "$file" ]]; then
    echo "::error::$context file not found"
    exit 1
  fi
  message="$(tr -d '\r' <"$file")"
  if ! check_message "$context" "$message" "multi"; then
    failed=1
  fi
  if ! check_version_ledger_entries "$context" "$message"; then
    failed=1
  fi
  check_manifest_versions
  check_lock_versions
  check_active_metadata_versions
  check_version_ledger_file
  exit "$failed"
fi

check_manifest_versions
check_lock_versions
check_active_metadata_versions
check_version_ledger_file
check_manifest_versions_match_latest_commit

for commit in "${commits[@]}"; do
  message="$(git log -1 --format=%B "$commit" | tr -d '\r')"
  subject="$(git log -1 --format=%s "$commit")"

  if ! check_message "commit $commit ($subject)" "$message" "single"; then
    failed=1
  fi
  if ! check_version_ledger_entries "commit $commit ($subject)" "$message"; then
    failed=1
  fi
done

check_commit_version_order

exit "$failed"
