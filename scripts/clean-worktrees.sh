#!/usr/bin/env bash
# Only Git's worktree inventory authorizes deletion; never scan agent directories.
set -euo pipefail
export GIT_OPTIONAL_LOCKS=0

dry_run=false
force=false
stale_days=3
usage() {
    echo 'Usage: clean-worktrees.sh [--dry-run] [--force] [--stale-days N]'
}
while (($#)); do
    case "$1" in
        --dry-run) dry_run=true ;;
        --force) force=true ;;
        --stale-days)
            shift
            [[ ${1:-} =~ ^[0-9]+$ && ${#1} -le 6 ]] || { usage >&2; exit 2; }
            stale_days=$((10#$1)) ;;
        --help|-h) usage; exit 0 ;;
        *) usage >&2; exit 2 ;;
    esac
    shift
done

current=$(git rev-parse --show-toplevel)
current=$(cd "$current" && pwd -P)
if "$dry_run"; then
    echo 'Dry run: cached refs only; fetch and prune skipped.' >&2
else
    git fetch --prune origin
fi
git rev-parse --verify origin/master >/dev/null

paths=() branches=() locked=()
index=-1
while IFS= read -r -d '' field; do
    case "$field" in
        'worktree '*)
            index=$((index + 1))
            paths[index]=${field#worktree }
            branches[index]=''
            locked[index]=false ;;
        'branch '*) branches[index]=${field#branch refs/heads/} ;;
        locked*) locked[index]=true ;;
    esac
done < <(git worktree list --porcelain -z)
((${#paths[@]})) || exit 1
main=${paths[0]}
gh_available=false
if command -v gh >/dev/null && gh auth status >/dev/null 2>&1; then
    gh_available=true
else
    echo 'Warning: gh missing or unauthenticated; skipping merged-PR lookup.' >&2
fi
origin=$(git remote get-url origin)
total=0
printf '%-60s  %-42s  %s\n' PATH ACTION 'FREED (KiB, du)'
report() {
    printf '%-60q  %-42s  %s\n' "$1" "$2" "${3:-0}"
}
# du -k reports allocated KiB on both macOS and Linux. Fail closed on errors.
size() {
    local result
    if [[ ! -e $1 && ! -L $1 ]]; then echo 0; return; fi
    result=$(du -sk "$1") || return 1
    read -r result _ <<< "$result"
    [[ $result =~ ^[0-9]+$ ]] || return 1
    echo "$result"
}
remove_worktree() {
    if "$force"; then
        git worktree remove --force -- "$1"
    else
        # Retain Git's own last-moment dirty check as well as ours. This repo
        # ignores target/, so build output does not require forcing removal.
        git worktree remove -- "$1"
    fi
}
for i in "${!paths[@]}"; do
    path=${paths[i]}
    branch=${branches[i]}
    if [[ $path == "$main" || $path == "$current" ]]; then
        report "$path" 'skip: main/current'; continue
    fi
    if "${locked[i]}"; then report "$path" 'skip: locked'; continue; fi
    if [[ ! -d $path || -L $path ]]; then report "$path" 'skip: missing/symlink'; continue; fi
    canonical=$(cd "$path" && pwd -P)
    if [[ $canonical != "$path" ]]; then report "$path" 'skip: noncanonical path'; continue; fi
    # Removing a parent would also remove another registered worktree.
    nested=false
    for other in "${paths[@]}"; do
        if [[ $other == "$path/"* ]]; then nested=true; fi
    done
    if "$nested"; then report "$path" 'skip: contains another worktree'; continue; fi
    # Refuse replaced directories that now belong to another repo/worktree.
    actual=$(git -C "$path" rev-parse --show-toplevel 2>/dev/null) || {
        report "$path" 'skip: invalid worktree'; continue;
    }
    common=$(git -C "$path" rev-parse --path-format=absolute --git-common-dir)
    own_common=$(git rev-parse --path-format=absolute --git-common-dir)
    if [[ $actual != "$path" || $common != "$own_common" ]]; then
        report "$path" 'skip: worktree identity changed'; continue
    fi
    merged=false
    if [[ -n $branch ]]; then
        if git merge-base --is-ancestor "refs/heads/$branch" origin/master; then
            merged=true
        else
            upstream=$(git for-each-ref --format='%(upstream)' "refs/heads/$branch")
            if [[ $upstream == refs/remotes/origin/* ]] && ! git show-ref --verify --quiet "$upstream"; then
                merged=true
            elif "$gh_available"; then
                if prs=$(gh pr list --repo "$origin" --state merged --head "$branch" --json number --jq 'length' 2>/dev/null); then
                    if [[ $prs =~ ^[0-9]+$ && $prs -gt 0 ]]; then merged=true; fi
                else
                    echo "Warning: merged-PR lookup failed for $branch; using Git evidence only." >&2
                fi
            fi
        fi
    fi
    if "$merged"; then
        # Include ignored files (e.g. local credentials), but exclude build output.
        dirty=$(git -C "$path" status --porcelain --untracked-files=all --ignored --ignore-submodules=none -- . ':(top,exclude)target')
        if [[ -n $dirty ]] && ! "$force"; then
            echo "Warning: dirty worktree skipped: $path (use --force to discard changes)." >&2
            report "$path" 'skip: dirty'; continue
        fi
        if "$dry_run"; then report "$path" 'would remove worktree + branch'; continue; fi
        before=$(size "$path")
        if remove_worktree "$path"; then
            after=$(size "$path")
            freed=$((before - after))
            total=$((total + freed))
            report "$path" 'removed worktree' "$freed"
            git branch -D -- "$branch" >&2 || echo "Warning: could not delete branch $branch" >&2
        else
            report "$path" 'remove failed'
        fi
    else
        git_dir=$(git -C "$path" rev-parse --absolute-git-dir)
        # find does not follow symlinks. Any recent file/directory or Git metadata
        # (including the index and HEAD reflog) counts as activity.
        recent=$(find "$path" "$git_dir" -mtime "-$stale_days" -print) || {
            report "$path" 'skip: cannot determine activity'; continue;
        }
        if [[ -n $recent ]]; then report "$path" 'keep: active'; continue; fi
        target=$path/target
        if [[ ! -d $target || -L $target ]]; then report "$path" 'keep: no safe target directory'; continue; fi
        tracked=$(git -C "$path" ls-files -- target)
        if [[ -n $tracked ]]; then report "$path" 'skip: target contains tracked files'; continue; fi
        if "$dry_run"; then report "$path" 'would remove stale target only'; continue; fi
        before=$(size "$target")
        rm -rf -- "$target"
        after=$(size "$target")
        freed=$((before - after))
        total=$((total + freed))
        report "$path" 'removed stale target only' "$freed"
    fi
done
if ! "$dry_run"; then git worktree prune; fi
printf 'Total disk freed: %s KiB (du before minus after)\n' "$total"
