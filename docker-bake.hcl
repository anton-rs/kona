variable "REGISTRY" {
  default = "ghcr.io"
}

variable "REPOSITORY" {
  default = "op-rs/kona"
}

variable "DEFAULT_TAG" {
  default = "kona:local"
}

variable "PLATFORMS" {
  // Only specify a single platform when `--load` ing into docker.
  // Multi-platform is supported when outputting to disk or pushing to a registry.
  // Multi-platform builds can be tested locally with:  --set="*.output=type=image,push=false"
  default = "linux/amd64,linux/arm64"
}

variable "GIT_REF_NAME" {
  default = "dev"
}

variable "ASTERISC_TAG" {
  // The tag of `asterisc` to use in the `kona-fpp-asterisc` target.
  //
  // You can override this if you'd like to use a different tag to generate the prestate.
  // https://github.com/ethereum-optimism/asterisc/releases
  default = "v1.2.0"
}

// Special target: https://github.com/docker/metadata-action#bake-definition
target "docker-metadata-action" {
  tags = ["${DEFAULT_TAG}"]
}

target "asterisc-builder" {
  inherits = ["docker-metadata-action"]
  context = "build/asterisc"
  dockerfile = "asterisc.dockerfile"
  platforms = split(",", PLATFORMS)
}

target "cannon-builder" {
  inherits = ["docker-metadata-action"]
  context = "build/cannon"
  dockerfile = "cannon.dockerfile"
  platforms = split(",", PLATFORMS)
}

target "kona-fpp-asterisc" {
  inherits = ["docker-metadata-action"]
  context = "."
  dockerfile = "build/asterisc/asterisc-repro.dockerfile"
  args = {
    CLIENT_TAG = "${GIT_REF_NAME}"
    ASTERISC_TAG = "${ASTERISC_TAG}"
  }
  platforms = split(",", PLATFORMS)
}
