# `docker`

This directory contains all of the repositories' dockerfiles as well as the [bake file](https://docs.docker.com/build/bake/)
used to define this repository's docker build configuration.

## Install Dependencies

* `docker`: https://www.docker.com/get-started/
* `docker-buildx`: https://github.com/docker/buildx?tab=readme-ov-file#installing

## Building Locally

To build any image in the bake file locally, use `docker bake`:

```sh
export TARGET="<target_name>"

# Optional: adjust the tag for the image
# Defaults to `kona:local`
export DEFAULT_TAG="my-image:local"

# Optional: Override the platforms to build the image for.
# Defaults to `linux/amd64,linux/arm64`
export PLATFORMS="<platforms>"

# Optional: Override the git ref to use for the current repo. Must exist
# on the `op-rs/kona` remote.
#
# Used by:
# - `kona-host`
# - `kona-asterisc-prestate`
export GIT_REF_NAME="my/feature/branch"

docker buildx bake \
  --progress plain \
  -f docker/docker-bake.hcl \
  $TARGET
```

## Cutting a Release (for maintainers / forks)

To cut a release of the docker image for any of the targets, cut a new annotated tag for the target like so:

```sh
# Example formats:
# - `kona-host/v0.1.0-beta.8`
# - `cannon-builder/v1.2.0`
TAG="<target_name>/<version>"
git tag -a $TAG -m "<tag description>" && git push origin tag $TAG
```

To run the workflow manually, navigate over to the ["Build and Publish Docker Image"](https://github.com/op-rs/kona/actions/workflows/docker.yaml)
action. From there, run a `workflow_dispatch` trigger, select the tag you just pushed, and then finally select the image to release.

Or, if you prefer to use the `gh` CLI, you can run:
```sh
gh workflow run "Build and Publish Docker Image" --ref <tag> -f image_to_release=<target>
```
