#!/usr/bin/env bash
set -euo pipefail

require_env() {
  local name="$1"
  if [[ -z "${!name:-}" ]]; then
    echo "${name} is required." >&2
    exit 1
  fi
}

require_env GITEE_USERNAME
require_env GITEE_TOKEN
require_env GITEE_REPOSITORY
require_env GITHUB_REPOSITORY
require_env GH_TOKEN

SYNC_PREFIX="${SYNC_PREFIX:-sync/gitee}"
PROTECTED_BRANCHES="${PROTECTED_BRANCHES:-main codex-runtime-device-abi}"
SYNC_RETRY_ATTEMPTS="${SYNC_RETRY_ATTEMPTS:-3}"
SYNC_RETRY_DELAY_SECS="${SYNC_RETRY_DELAY_SECS:-10}"
SYNC_GIT_TIMEOUT_SECS="${SYNC_GIT_TIMEOUT_SECS:-120}"

git config user.name "github-actions[bot]"
git config user.email "41898282+github-actions[bot]@users.noreply.github.com"

git remote remove gitee 2>/dev/null || true
git remote add gitee "https://${GITEE_USERNAME}:${GITEE_TOKEN}@gitee.com/${GITEE_REPOSITORY}.git"

git_net() {
  timeout "${SYNC_GIT_TIMEOUT_SECS}" \
    git \
      -c http.lowSpeedLimit=1 \
      -c http.lowSpeedTime=30 \
      "$@"
}

retry() {
  local attempt=1
  local status=0

  while true; do
    if "$@"; then
      return 0
    fi

    status=$?
    if (( attempt >= SYNC_RETRY_ATTEMPTS )); then
      echo "Command failed after ${attempt} attempt(s): $*" >&2
      return "${status}"
    fi

    echo "Command failed with status ${status}; retrying in ${SYNC_RETRY_DELAY_SECS}s: $*" >&2
    sleep "${SYNC_RETRY_DELAY_SECS}"
    attempt=$((attempt + 1))
  done
}

echo "Fetching GitHub and Gitee branch refs..."
retry git_net fetch origin '+refs/heads/*:refs/remotes/origin/*' --prune
retry git_net fetch gitee '+refs/heads/*:refs/remotes/gitee/*' --prune
retry git_net fetch origin '+refs/tags/*:refs/tags/*' --prune || true

branch_ref_exists() {
  git show-ref --verify --quiet "$1"
}

is_ancestor() {
  git merge-base --is-ancestor "$1" "$2"
}

same_tree() {
  [[ "$(git rev-parse "$1^{tree}")" == "$(git rev-parse "$2^{tree}")" ]]
}

is_protected_branch() {
  local branch="$1"
  for protected in ${PROTECTED_BRANCHES}; do
    if [[ "${branch}" == "${protected}" ]]; then
      return 0
    fi
  done
  return 1
}

skip_branch() {
  local branch="$1"
  [[ "${branch}" == "HEAD" || "${branch}" == "${SYNC_PREFIX}/"* ]]
}

open_or_update_gitee_pr() {
  local branch="$1"
  local reason="$2"
  local sync_branch="${SYNC_PREFIX}/${branch}"
  local title="sync: import Gitee ${branch}"

  echo "Publishing ${sync_branch} for ${branch}: ${reason}"
  retry git_net push origin "refs/remotes/gitee/${branch}:refs/heads/${sync_branch}"

  local existing_pr
  existing_pr="$(gh pr list \
    --repo "${GITHUB_REPOSITORY}" \
    --state open \
    --head "${sync_branch}" \
    --base "${branch}" \
    --json number \
    --jq '.[0].number // empty')"

  if [[ -n "${existing_pr}" ]]; then
    echo "PR #${existing_pr} already tracks Gitee ${branch}."
    return 0
  fi

  gh pr create \
    --repo "${GITHUB_REPOSITORY}" \
    --base "${branch}" \
    --head "${sync_branch}" \
    --title "${title}" \
    --body "This PR was opened by the GitHub/Gitee sync workflow.\n\nReason: ${reason}\n\nThe workflow never force-pushes over contributor work. Review and merge this PR to make GitHub include the Gitee-side commits; a later sync can then fast-forward Gitee again."
}

sync_gitee_branches_into_github() {
  echo "Importing Gitee branch updates into GitHub..."
  while IFS= read -r ref; do
    local branch="${ref#gitee/}"
    if skip_branch "${branch}"; then
      continue
    fi

    local gitee_ref="refs/remotes/gitee/${branch}"
    local github_ref="refs/remotes/origin/${branch}"

    if ! branch_ref_exists "${github_ref}"; then
      echo "GitHub is missing ${branch}; creating it from Gitee."
      retry git_net push origin "${gitee_ref}:refs/heads/${branch}"
      continue
    fi

    local gitee_sha
    local github_sha
    gitee_sha="$(git rev-parse "${gitee_ref}")"
    github_sha="$(git rev-parse "${github_ref}")"

    if [[ "${gitee_sha}" == "${github_sha}" ]]; then
      continue
    fi

    if same_tree "${github_ref}" "${gitee_ref}"; then
      echo "GitHub and Gitee content already match for ${branch}; no import PR needed."
      continue
    fi

    if is_ancestor "${github_ref}" "${gitee_ref}"; then
      if is_protected_branch "${branch}"; then
        open_or_update_gitee_pr "${branch}" "Gitee is ahead of protected GitHub branch ${branch}."
      else
        echo "Fast-forwarding GitHub ${branch} from Gitee."
        if ! retry git_net push origin "${gitee_ref}:refs/heads/${branch}"; then
          open_or_update_gitee_pr "${branch}" "GitHub rejected a fast-forward update for ${branch}."
        fi
      fi
      continue
    fi

    if is_ancestor "${gitee_ref}" "${github_ref}"; then
      echo "GitHub ${branch} is ahead of Gitee; outbound sync will handle it."
      continue
    fi

    open_or_update_gitee_pr "${branch}" "GitHub and Gitee have diverged on ${branch}."
  done < <(git for-each-ref --format='%(refname:short)' refs/remotes/gitee)
}

sync_github_branches_into_gitee() {
  echo "Sending GitHub branch updates to Gitee when safe..."
  while IFS= read -r ref; do
    local branch="${ref#origin/}"
    if skip_branch "${branch}"; then
      continue
    fi

    local github_ref="refs/remotes/origin/${branch}"
    local gitee_ref="refs/remotes/gitee/${branch}"

    if ! branch_ref_exists "${gitee_ref}"; then
      echo "Gitee is missing ${branch}; creating it from GitHub."
      retry git_net push gitee "${github_ref}:refs/heads/${branch}"
      continue
    fi

    local gitee_sha
    local github_sha
    gitee_sha="$(git rev-parse "${gitee_ref}")"
    github_sha="$(git rev-parse "${github_ref}")"

    if [[ "${gitee_sha}" == "${github_sha}" ]]; then
      continue
    fi

    if same_tree "${gitee_ref}" "${github_ref}"; then
      echo "GitHub and Gitee content already match for ${branch}; skipping outbound push."
      continue
    fi

    if is_ancestor "${gitee_ref}" "${github_ref}"; then
      echo "Fast-forwarding Gitee ${branch} from GitHub."
      retry git_net push gitee "${github_ref}:refs/heads/${branch}"
      continue
    fi

    if is_ancestor "${github_ref}" "${gitee_ref}"; then
      echo "Gitee ${branch} is ahead of GitHub; inbound sync/PR handles it."
      continue
    fi

    echo "GitHub and Gitee have diverged on ${branch}; skipping outbound push."
  done < <(git for-each-ref --format='%(refname:short)' refs/remotes/origin)
}

tag_sha_from_file() {
  local file="$1"
  local tag="$2"
  awk -v ref="refs/tags/${tag}" '$2 == ref { print $1; exit }' "${file}"
}

sync_tags() {
  echo "Synchronizing missing tags without overwriting tag conflicts..."
  local origin_tags
  local gitee_tags
  origin_tags="$(mktemp)"
  gitee_tags="$(mktemp)"

  retry git_net ls-remote --tags origin > "${origin_tags}.all" || true
  retry git_net ls-remote --tags gitee > "${gitee_tags}.all" || true
  grep -v '\^{}' "${origin_tags}.all" > "${origin_tags}" || true
  grep -v '\^{}' "${gitee_tags}.all" > "${gitee_tags}" || true

  while read -r sha ref; do
    [[ -z "${sha:-}" || -z "${ref:-}" ]] && continue
    local tag="${ref#refs/tags/}"
    local gitee_sha
    gitee_sha="$(tag_sha_from_file "${gitee_tags}" "${tag}")"
    if [[ -z "${gitee_sha}" ]]; then
      echo "Creating missing Gitee tag ${tag}."
      retry git_net push gitee "refs/tags/${tag}:refs/tags/${tag}"
    elif [[ "${gitee_sha}" != "${sha}" ]]; then
      echo "Tag conflict for ${tag}; leaving both sides unchanged."
    fi
  done < "${origin_tags}"

  while read -r sha ref; do
    [[ -z "${sha:-}" || -z "${ref:-}" ]] && continue
    local tag="${ref#refs/tags/}"
    local origin_sha
    origin_sha="$(tag_sha_from_file "${origin_tags}" "${tag}")"
    if [[ -z "${origin_sha}" ]]; then
      echo "Importing missing GitHub tag ${tag} from Gitee."
      retry git_net fetch gitee "refs/tags/${tag}:refs/tags/${tag}"
      retry git_net push origin "refs/tags/${tag}:refs/tags/${tag}"
    elif [[ "${origin_sha}" != "${sha}" ]]; then
      echo "Tag conflict for ${tag}; leaving both sides unchanged."
    fi
  done < "${gitee_tags}"
}

sync_gitee_branches_into_github
sync_github_branches_into_gitee
sync_tags

echo "GitHub/Gitee synchronization completed."
