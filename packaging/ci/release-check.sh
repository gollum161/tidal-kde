#!/bin/bash
# Decides whether this CI run publishes a release (runs after all builds passed).
#  - push to main: release when the tag v<version from Cargo.toml> doesn't exist yet,
#    i.e. the version was raised. The release step then creates the tag on this commit.
#  - push of a tag v<version>: release (common.sh already checked the tag matches).
# Writes release=true|false and tag=v<version> to $GITHUB_OUTPUT.
. "$(dirname "$0")/common.sh"
rmdir dist 2>/dev/null || true

tag="v$VERSION"
if [ "${GITHUB_REF_TYPE:-}" = tag ]; then
	release=true
elif git ls-remote --exit-code --tags origin "refs/tags/$tag" >/dev/null; then
	echo "Tag $tag already exists – no new release (raise the version in Cargo.toml for one)"
	release=false
else
	echo "Version $VERSION has no tag yet – publishing release $tag"
	release=true
fi

echo "release=$release" >> "$GITHUB_OUTPUT"
echo "tag=$tag" >> "$GITHUB_OUTPUT"
