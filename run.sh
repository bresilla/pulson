#!/bin/bash


# @cmd build cargo project
# @alias b
build() {
    cargo build --release
}

# @cmd run cargo project
# @alias r
run() {
    PULSON_IP=172.30.0.175 ./target/release/pulson serve --webui --root-pass "superdupersecret"
}

# @cmd mark as releaser
# @arg type![patch|minor|major] Release type
release() {
    CURRENT_VERSION=$(grep '^version = ' pulson/Cargo.toml | sed -E 's/version = "(.*)"/\\1/')
    echo "Current version: $CURRENT_VERSION"
    IFS='.' read -r MAJOR MINOR PATCH <<< "$CURRENT_VERSION"
    case $argc_type in
        major)
            MAJOR=$((MAJOR + 1))
            MINOR=0
            PATCH=0
            ;;
        minor)
            MINOR=$((MINOR + 1))
            PATCH=0
            ;;
        patch)
            PATCH=$((PATCH + 1))
            ;;
    esac
    numeric_version="$MAJOR.$MINOR.$PATCH"
    tag_version="v$numeric_version"

    echo "New version: $numeric_version (tag: $tag_version)"

    # Update Cargo.toml files with the numeric version
    sed -i "s/^version = \".*\"/version = \"$numeric_version\"/" pulson/Cargo.toml
    sed -i "s/^version = \".*\"/version = \"$numeric_version\"/" pulson-ui/Cargo.toml

    # Generate changelog for the new v-prefixed tag
    # This command generates/updates the entire CHANGELOG.md file
    git cliff --tag "$tag_version" > CHANGELOG.md

    # Get the changelog content for the current release (unreleased changes before tagging)
    changelog_content=$(git cliff --unreleased --strip all)

    git add -A && git commit -m "chore(release): prepare for $tag_version"
    echo "$changelog_content"

    # Create the v-prefixed git tag
    git tag -a "$tag_version" -m "$tag_version" -m "$changelog_content"
    git push --follow-tags --force --set-upstream origin develop

    # Create GitHub release with the v-prefixed tag
    gh release create "$tag_version" --notes "$changelog_content"
}


# @cmd compile mdbook
# @alias m
# @option    --dest_dir <dir>    Destination directory
# @flag      --monitor        Monitor after upload
mdbook() {
    mdbook build book --dest-dir ../docs
    git add -A && git commit -m "docs: building website/mdbook"
}


eval "$(argc --argc-eval "$0" "$@")"
