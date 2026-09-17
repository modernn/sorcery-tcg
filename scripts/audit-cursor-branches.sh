#!/usr/bin/env bash
# Audit cursor/* lane branches against master catalog bindings.
# Usage:
#   ./scripts/audit-cursor-branches.sh
#   ./scripts/audit-cursor-branches.sh --list
#   ./scripts/audit-cursor-branches.sh --delete-remote

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

git fetch origin --prune >/dev/null 2>&1 || true

catalog_file="$repo_root/data/rules/catalog.json"
mode="${1-}"

has_catalog_id() {
  local id="$1"
  rg -q "\"ruleId\": \"RULE-CATALOG-${id}\"" "$catalog_file" 2>/dev/null
}

branch_catalog_id() {
  local branch="$1"
  local from_name from_msg
  from_name="$(echo "$branch" | grep -oE '[0-9]{4}' | head -1 || true)"
  from_msg="$(git log -1 --format='%s' "origin/$branch" 2>/dev/null | grep -oE '[0-9]{4}' | head -1 || true)"
  if [[ -n "$from_msg" ]]; then
    echo "$from_msg"
  else
    echo "$from_name"
  fi
}

classify() {
  local branch="$1"
  if [[ "$branch" == "cursor/source-bound-control-0005" ]]; then
    echo "KEEP"
    return
  fi
  if git merge-base --is-ancestor "origin/$branch" master 2>/dev/null; then
    echo "DELETE_MERGED"
    return
  fi
  local id
  id="$(branch_catalog_id "$branch")"
  if [[ -n "$id" ]] && has_catalog_id "$id"; then
    echo "DELETE_LANDED"
    return
  fi
  echo "REVIEW"
}

mapfile -t branches < <(git branch -r --format='%(refname:short)' | sed 's|^origin/||' | grep '^cursor/' | sort -u)

delete_merged=0
delete_landed=0
keep=0
review=0
to_delete=()

for branch in "${branches[@]}"; do
  kind="$(classify "$branch")"
  case "$kind" in
    DELETE_MERGED)
      delete_merged=$((delete_merged + 1))
      to_delete+=("$branch")
      ;;
    DELETE_LANDED)
      delete_landed=$((delete_landed + 1))
      to_delete+=("$branch")
      ;;
    KEEP) keep=$((keep + 1)) ;;
    REVIEW)
      review=$((review + 1))
      if [[ "$mode" == "--list" ]]; then
        ahead="$(git rev-list --count master.."origin/$branch" 2>/dev/null || echo "?")"
        msg="$(git log -1 --format='%s' "origin/$branch" 2>/dev/null || true)"
        echo -e "REVIEW\t$branch\tahead=$ahead\t$msg"
      fi
      ;;
  esac
  if [[ "$mode" == "--list" && "$kind" != REVIEW ]]; then
    echo -e "$kind\t$branch"
  fi
done

if [[ "$mode" == "--delete-remote" ]]; then
  echo "Delete ${#to_delete[@]} remote cursor/* branches (merged or catalog-landed on master):"
  for branch in "${to_delete[@]}"; do
    echo "git push origin --delete $branch"
  done
  exit 0
fi

worktrees="$(git worktree list 2>/dev/null | wc -l | tr -d ' ')"
echo "Remote cursor/* branches: ${#branches[@]}"
echo "Local worktrees: $worktrees"
echo "master @ $(git rev-parse --short master)"
echo "canonical @ $(git rev-parse --short cursor/source-bound-control-0005 2>/dev/null || echo missing)"
echo
echo "DELETE merged (git ancestor):     $delete_merged"
echo "DELETE landed (catalog on master): $delete_landed"
echo "KEEP canonical:                    $keep"
echo "REVIEW (unique / infra):           $review"
echo
echo "You do NOT need one PR per lane. Batch-merge canonical -> master, then bulk-delete landed lanes."
echo "Run: ./scripts/audit-cursor-branches.sh --list"
echo "Run: ./scripts/audit-cursor-branches.sh --delete-remote  # prints delete commands only"
