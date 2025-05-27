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
    sed -i "s/^version = \".*\"/version = \"$version\"/" Cargo.toml
    git cliff --tag $version > CHANGELOG.md
    changelog=$(git cliff --unreleased --strip all)
    git add -A && git commit -m "chore(release): prepare for $version"
    echo "$changelog"
    git tag -a $version -m "$version" -m "$changelog"
    git push --follow-tags --force --set-upstream origin main
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
