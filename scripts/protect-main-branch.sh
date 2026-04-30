#!/usr/bin/env bash
# Apply branch-protection rules to main on GitHub.
#
# Usage:
#   GITHUB_TOKEN=<your_pat> ./scripts/protect-main-branch.sh
#
# The token needs the "repo" scope (or "Administration: write" for fine-grained PATs).
# Generate one at: https://github.com/settings/tokens
set -euo pipefail

REPO="dc0sk/rusty-wire"
BRANCH="main"

if [[ -z "${GITHUB_TOKEN:-}" ]]; then
  echo "Error: GITHUB_TOKEN is not set." >&2
  echo "Export a GitHub personal-access token with the 'repo' scope and re-run." >&2
  exit 1
fi

echo "Applying branch protection for '${BRANCH}' on ${REPO} ..."

curl -fsSL \
  -X PUT \
  -H "Accept: application/vnd.github+json" \
  -H "Authorization: Bearer ${GITHUB_TOKEN}" \
  -H "X-GitHub-Api-Version: 2022-11-28" \
  "https://api.github.com/repos/${REPO}/branches/${BRANCH}/protection" \
  -d '{
    "required_status_checks": null,
    "enforce_admins": true,
    "required_pull_request_reviews": {
      "dismiss_stale_reviews": false,
      "require_code_owner_reviews": false,
      "required_approving_review_count": 0
    },
    "restrictions": null,
    "allow_force_pushes": false,
    "allow_deletions": false,
    "block_creations": false,
    "required_conversation_resolution": false
  }'

echo
echo "Done. Direct pushes to '${BRANCH}' are now blocked."
echo "Pull requests are required to merge into main."
