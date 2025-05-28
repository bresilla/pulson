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
    CURRENT_VERSION=$(grep '^version = ' pulson/Cargo.toml | sed -E 's/version = "(.*)"/\1/')
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
    version="$MAJOR.$MINOR.$PATCH"
    echo "New version: $version"
    
    # Get the latest tag to create a range for changelog generation
    LATEST_TAG=$(git tag --list --sort=-version:refname | head -n 1)
    if [ -n "$LATEST_TAG" ]; then
        # Get changelog content for release notes (changes since last tag)
        changelog=$(git cliff $LATEST_TAG..HEAD --strip all)
        # Generate changelog and prepend to existing file (changes since last tag)
        git cliff --tag $version $LATEST_TAG..HEAD --prepend CHANGELOG.md
    else
        # First release - get all changes
        changelog=$(git cliff --unreleased --strip all)
        git cliff --tag $version --unreleased --prepend CHANGELOG.md
    fi
    # Update version in Cargo.toml files
    sed -i "s/^version = \".*\"/version = \"$version\"/" pulson/Cargo.toml
    sed -i "s/^version = \".*\"/version = \"$version\"/" pulson-ui/Cargo.toml
    git add -A && git commit -m "chore(release): prepare for $version"
    echo "$changelog"
    git tag -a $version -m "$version" -m "$changelog"
    git push --follow-tags --force --set-upstream origin develop
    gh release create $version --notes "$changelog"
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
